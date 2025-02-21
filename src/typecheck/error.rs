//! Internal error types for typechecking.
use super::{reporting, State, TypeWrapper};
use crate::{
    error::TypecheckError,
    identifier::Ident,
    label::ty_path,
    position::TermPos,
    term::RichTerm,
    types::{AbsType, Types},
};

/// Error during the unification of two row types.
#[derive(Debug, PartialEq)]
pub enum RowUnifError {
    /// The LHS had a binding that was missing in the RHS.
    MissingRow(Ident),
    /// The LHS had a `Dyn` tail that was missing in the RHS.
    MissingDynTail(),
    /// The RHS had a binding that was not in the LHS.
    ExtraRow(Ident),
    /// The RHS had a additional `Dyn` tail.
    ExtraDynTail(),
    /// There were two incompatible definitions for the same row.
    RowMismatch(Ident, Box<UnifError>),
    /// Tried to unify an enum row and a record row.
    RowKindMismatch(Ident, Option<TypeWrapper>, Option<TypeWrapper>),
    /// One of the row was ill-formed (typically, a tail was neither a row, a variable nor `Dyn`).
    ///
    /// This should probably not happen with proper restrictions on the parser and a correct
    /// typechecking algorithm. We let it as an error for now, but it could be removed in the
    /// future.
    IllformedRow(TypeWrapper),
    /// A [row constraint](./type.RowConstr.html) was violated.
    UnsatConstr(Ident, Option<TypeWrapper>),
    /// Tried to unify a type constant with another different type.
    WithConst(usize, TypeWrapper),
    /// Tried to unify two distinct type constants.
    ConstMismatch(usize, usize),
    /// An unbound type variable was referenced.
    UnboundTypeVariable(Ident),
}

impl RowUnifError {
    /// Convert a row unification error to a unification error.
    ///
    /// There is a hierarchy between error types, from the most local/specific to the most high-level:
    /// - [`RowUnifError`](./enum.RowUnifError.html)
    /// - [`UnifError`](./enum.UnifError.html)
    /// - [`TypecheckError`](../errors/enum.TypecheckError.html)
    ///
    /// Each level usually adds information (such as types or positions) and group different
    /// specific errors into most general ones.
    pub fn into_unif_err(self, left: TypeWrapper, right: TypeWrapper) -> UnifError {
        match self {
            RowUnifError::MissingRow(id) => UnifError::MissingRow(id, left, right),
            RowUnifError::MissingDynTail() => UnifError::MissingDynTail(left, right),
            RowUnifError::ExtraRow(id) => UnifError::ExtraRow(id, left, right),
            RowUnifError::ExtraDynTail() => UnifError::ExtraDynTail(left, right),
            RowUnifError::RowKindMismatch(id, tyw1, tyw2) => {
                UnifError::RowKindMismatch(id, tyw1, tyw2)
            }
            RowUnifError::RowMismatch(id, err) => UnifError::RowMismatch(id, left, right, err),
            RowUnifError::IllformedRow(tyw) => UnifError::IllformedRow(tyw),
            RowUnifError::UnsatConstr(id, tyw) => UnifError::RowConflict(id, tyw, left, right),
            RowUnifError::WithConst(c, tyw) => UnifError::WithConst(c, tyw),
            RowUnifError::ConstMismatch(c1, c2) => UnifError::ConstMismatch(c1, c2),
            RowUnifError::UnboundTypeVariable(id) => UnifError::UnboundTypeVariable(id),
        }
    }
}

/// Error during the unification of two types.
#[derive(Debug, PartialEq)]
pub enum UnifError {
    /// Tried to unify two incompatible types.
    TypeMismatch(TypeWrapper, TypeWrapper),
    /// There are two incompatible definitions for the same row.
    RowMismatch(Ident, TypeWrapper, TypeWrapper, Box<UnifError>),
    /// Tried to unify an enum row and a record row.
    RowKindMismatch(Ident, Option<TypeWrapper>, Option<TypeWrapper>),
    /// Tried to unify two distinct type constants.
    ConstMismatch(usize, usize),
    /// Tried to unify two rows, but an identifier of the LHS was absent from the RHS.
    MissingRow(Ident, TypeWrapper, TypeWrapper),
    /// Tried to unify two rows, but the `Dyn` tail of the RHS was absent from the LHS.
    MissingDynTail(TypeWrapper, TypeWrapper),
    /// Tried to unify two rows, but an identifier of the RHS was absent from the LHS.
    ExtraRow(Ident, TypeWrapper, TypeWrapper),
    /// Tried to unify two rows, but the `Dyn` tail of the RHS was absent from the LHS.
    ExtraDynTail(TypeWrapper, TypeWrapper),
    /// A row was ill-formed.
    IllformedRow(TypeWrapper),
    /// Tried to unify a unification variable with a row type violating the [row
    /// constraints](./type.RowConstr.html) of the variable.
    RowConflict(Ident, Option<TypeWrapper>, TypeWrapper, TypeWrapper),
    /// Tried to unify a type constant with another different type.
    WithConst(usize, TypeWrapper),
    /// A flat type, which is an opaque type corresponding to custom contracts, contained a Nickel
    /// term different from a variable. Only a variables is a legal inner term of a flat type.
    IllformedFlatType(RichTerm),
    /// A generic type was ill-formed. Currently, this happens if a `StatRecord` or `Enum` type
    /// does not contain a row type.
    IllformedType(TypeWrapper),
    /// An unbound type variable was referenced.
    UnboundTypeVariable(Ident),
    /// An error occurred when unifying the domains of two arrows.
    DomainMismatch(TypeWrapper, TypeWrapper, Box<UnifError>),
    /// An error occurred when unifying the codomains of two arrows.
    CodomainMismatch(TypeWrapper, TypeWrapper, Box<UnifError>),
}

impl UnifError {
    /// Convert a unification error to a typechecking error.
    ///
    /// Wrapper that calls [`to_typecheck_err_`](./fn.to_typecheck_err_.html) with an empty [name
    /// registry](./reporting/struct.NameReg.html).
    pub fn into_typecheck_err(self, state: &State, pos_opt: TermPos) -> TypecheckError {
        self.into_typecheck_err_(state, &mut reporting::NameReg::new(), pos_opt)
    }

    /// Convert a unification error to a typechecking error.
    ///
    /// There is a hierarchy between error types, from the most local/specific to the most high-level:
    /// - [`RowUnifError`](./enum.RowUnifError.html)
    /// - [`UnifError`](./enum.UnifError.html)
    /// - [`TypecheckError`](../errors/enum.TypecheckError.html)
    ///
    /// Each level usually adds information (such as types or positions) and group different
    /// specific errors into most general ones.
    ///
    /// # Parameters
    ///
    /// - `state`: the state of unification. Used to access the unification table, and the original
    /// names of of unification variable or type constant.
    /// - `names`: a [name registry](./reporting/struct.NameReg.html), structure used to assign
    /// unique a humain-readable names to unification variables and type constants.
    /// - `pos_opt`: the position span of the expression that failed to typecheck.
    pub fn into_typecheck_err_(
        self,
        state: &State,
        names: &mut reporting::NameReg,
        pos_opt: TermPos,
    ) -> TypecheckError {
        match self {
            UnifError::TypeMismatch(ty1, ty2) => TypecheckError::TypeMismatch(
                reporting::to_type(state.table, state.names, names, ty1),
                reporting::to_type(state.table, state.names, names, ty2),
                pos_opt,
            ),
            UnifError::RowMismatch(ident, tyw1, tyw2, err) => TypecheckError::RowMismatch(
                ident,
                reporting::to_type(state.table, state.names, names, tyw1),
                reporting::to_type(state.table, state.names, names, tyw2),
                Box::new((*err).into_typecheck_err_(state, names, TermPos::None)),
                pos_opt,
            ),
            UnifError::RowKindMismatch(id, ty1, ty2) => TypecheckError::RowKindMismatch(
                id,
                ty1.map(|tw| reporting::to_type(state.table, state.names, names, tw)),
                ty2.map(|tw| reporting::to_type(state.table, state.names, names, tw)),
                pos_opt,
            ),
            // TODO: for now, failure to unify with a type constant causes the same error as a
            // usual type mismatch. It could be nice to have a specific error message in the
            // future.
            UnifError::ConstMismatch(c1, c2) => TypecheckError::TypeMismatch(
                reporting::to_type(state.table, state.names, names, TypeWrapper::Constant(c1)),
                reporting::to_type(state.table, state.names, names, TypeWrapper::Constant(c2)),
                pos_opt,
            ),
            UnifError::WithConst(c, ty) => TypecheckError::TypeMismatch(
                reporting::to_type(state.table, state.names, names, TypeWrapper::Constant(c)),
                reporting::to_type(state.table, state.names, names, ty),
                pos_opt,
            ),
            UnifError::IllformedFlatType(rt) => {
                TypecheckError::IllformedType(Types(AbsType::Flat(rt)))
            }
            UnifError::IllformedType(tyw) => TypecheckError::IllformedType(reporting::to_type(
                state.table,
                state.names,
                names,
                tyw,
            )),
            UnifError::MissingRow(id, tyw1, tyw2) => TypecheckError::MissingRow(
                id,
                reporting::to_type(state.table, state.names, names, tyw1),
                reporting::to_type(state.table, state.names, names, tyw2),
                pos_opt,
            ),
            UnifError::MissingDynTail(tyw1, tyw2) => TypecheckError::MissingDynTail(
                reporting::to_type(state.table, state.names, names, tyw1),
                reporting::to_type(state.table, state.names, names, tyw2),
                pos_opt,
            ),
            UnifError::ExtraRow(id, tyw1, tyw2) => TypecheckError::ExtraRow(
                id,
                reporting::to_type(state.table, state.names, names, tyw1),
                reporting::to_type(state.table, state.names, names, tyw2),
                pos_opt,
            ),
            UnifError::ExtraDynTail(tyw1, tyw2) => TypecheckError::ExtraDynTail(
                reporting::to_type(state.table, state.names, names, tyw1),
                reporting::to_type(state.table, state.names, names, tyw2),
                pos_opt,
            ),
            UnifError::IllformedRow(tyw) => TypecheckError::IllformedType(reporting::to_type(
                state.table,
                state.names,
                names,
                tyw,
            )),
            UnifError::RowConflict(id, tyw, left, right) => TypecheckError::RowConflict(
                id,
                tyw.map(|tyw| reporting::to_type(state.table, state.names, names, tyw)),
                reporting::to_type(state.table, state.names, names, left),
                reporting::to_type(state.table, state.names, names, right),
                pos_opt,
            ),
            UnifError::UnboundTypeVariable(ident) => {
                TypecheckError::UnboundTypeVariable(ident, pos_opt)
            }
            err @ UnifError::CodomainMismatch(_, _, _)
            | err @ UnifError::DomainMismatch(_, _, _) => {
                let (expd, actual, path, err_final) = err.into_type_path().unwrap();
                TypecheckError::ArrowTypeMismatch(
                    reporting::to_type(state.table, state.names, names, expd),
                    reporting::to_type(state.table, state.names, names, actual),
                    path,
                    Box::new(err_final.into_typecheck_err_(state, names, TermPos::None)),
                    pos_opt,
                )
            }
        }
    }

    /// Transform a `(Co)DomainMismatch` into a type path and other data.
    ///
    /// `(Co)DomainMismatch` can be nested: when unifying `Num -> Num -> Num` with `Num -> Bool ->
    /// Num`, the resulting error is of the form `CodomainMismatch(.., DomainMismatch(..,
    /// TypeMismatch(..)))`. The heading sequence of `(Co)DomainMismatch` is better represented as
    /// a type path, here `[Codomain, Domain]`, while the last error of the chain -- which thus
    /// cannot be a `(Co)DomainMismatch` -- is the actual cause of the unification failure.
    ///
    /// This function breaks down a `(Co)Domain` mismatch into a more convenient representation.
    ///
    /// # Return
    ///
    /// Return `None` if `self` is not a `DomainMismatch` nor a `CodomainMismatch`.
    ///
    /// Otherwise, return the following tuple:
    ///  - the original expected type.
    ///  - the original actual type.
    ///  - a type path pointing at the subtypes which failed to be unified.
    ///  - the final error, which is the actual cause of that failure.
    pub fn into_type_path(self) -> Option<(TypeWrapper, TypeWrapper, ty_path::Path, Self)> {
        let mut curr: Self = self;
        let mut path = ty_path::Path::new();
        // The original expected and actual type. They are just updated once, in the first
        // iteration of the loop below.
        let mut tyws: Option<(TypeWrapper, TypeWrapper)> = None;

        loop {
            match curr {
                UnifError::DomainMismatch(
                    tyw1 @ TypeWrapper::Concrete(AbsType::Arrow(_, _)),
                    tyw2 @ TypeWrapper::Concrete(AbsType::Arrow(_, _)),
                    err,
                ) => {
                    tyws = tyws.or(Some((tyw1, tyw2)));
                    path.push(ty_path::Elem::Domain);
                    curr = *err;
                }
                UnifError::DomainMismatch(_, _, _) => panic!(
                    "typechecking::to_type_path(): domain mismatch error on a non arrow type"
                ),
                UnifError::CodomainMismatch(
                    tyw1 @ TypeWrapper::Concrete(AbsType::Arrow(_, _)),
                    tyw2 @ TypeWrapper::Concrete(AbsType::Arrow(_, _)),
                    err,
                ) => {
                    tyws = tyws.or(Some((tyw1, tyw2)));
                    path.push(ty_path::Elem::Codomain);
                    curr = *err;
                }
                UnifError::CodomainMismatch(_, _, _) => panic!(
                    "typechecking::to_type_path(): codomain mismatch error on a non arrow type"
                ),
                // tyws equals to `None` iff we did not even enter the case above once, i.e. if
                // `self` was indeed neither a `DomainMismatch` nor a `CodomainMismatch`
                _ => break tyws.map(|(expd, actual)| (expd, actual, path, curr)),
            }
        }
    }
}
