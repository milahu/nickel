let Assert = fun l x => x || %blame% l in

[
  let AlwaysTrue = fun l t =>
    let boolT | Bool = t in
    if boolT then boolT else %blame% l in
  let AlwaysFalse = fun l t =>
    let boolT | Bool = t in
    if boolT then %blame% l else boolT
  in let not | AlwaysTrue -> AlwaysFalse = fun b => if b then false else true in
  not true == false,

  # id_polymorphic_contract
  let id | forall a. a -> a = fun x => x in id true,

  # higher_order_contract
  let to_bool | forall a. (a -> Bool) -> a -> Bool =
    fun f => fun x => f x in
    to_bool (fun x => true) 4,

  # apply_twice
  let twice | forall a. (a -> a) -> a -> a =
    fun f => fun x => f (f x) in
  twice (fun x => x + 1) 3 == 5,

  # strings
  ("hello" | Str) == "hello",
  ("hello" ++ " world" | Str) == "hello world",

  # enums_simple
  (`foo | [| foo, bar |]) == `foo,
  (`bar | forall r. [| foo, bar ; r|]) == `bar,

  # enums_escaped
  (`"foo:baz" | [| "foo:baz", "bar:baz" |]) == `"foo:baz",
  (`"bar:baz" | forall r. [| "foo:baz", "bar:baz" ; r |]) == `"bar:baz",

  # enums_complex
  let f : forall r. [| foo, bar ; r |] -> Num =
    fun x => switch { `foo => 1, `bar => 2, _ => 3, } x in
  f `bar == 2,

  let f : forall r. [| foo, bar ; r |] -> Num =
    fun x => switch { `foo => 1, `bar => 2, _ => 3, } x in
  f `boo == 3,

  let f : forall r. [| foo, "bar:baz" ; r |] -> Num =
    fun x => switch { `foo => 1, `"bar:baz" => 2, _ => 3, } x in
  f `"bar:baz" == 2,

  let f : forall r. [| foo, "bar:baz" ; r |] -> Num =
    fun x => switch { `foo => 1, `"bar:baz" => 2, _ => 3, } x in
  f `"boo,grr" == 3,

  # enums_applied
  # Regression test for enum types converted to contracts outside of annotations
  # causing issues wrt typechecking
  let Wrapper = contract.apply [| Foo, Bar |] in
  (`Foo | Wrapper) == `Foo,

  # records_simple
  ({} | {}) == {},
  let x | {a: Num, s: Str} = {a = 1, s = "a"} in
    %deep_seq% x x == {a = 1, s = "a"},

  let x | {a: Num, s: {foo: Bool}} = {a = 1, s = { foo = true}} in
    %deep_seq% x x == {a = 1, s = { foo = true}},

  # polymorphism
  (let id | forall a. { ; a} -> { ; a} = fun x => x in
    let extend | forall a. { ; a} -> {foo: Num ; a} = fun x =>
      x & {foo = 1} in
    let remove | forall a. {foo: Num ; a} -> { ; a} = record.remove "foo" in

    (id {} == {} | Assert) &&
    (id {a = 1, b = false} == {a = 1, b = false} | Assert) &&
    (extend {} == {foo = 1} | Assert) &&
    (extend {bar = false} == {foo = 1, bar = false} | Assert) &&
    (remove {foo = 1} == {} | Assert) &&
    (remove {foo = 1, bar = 1} == {bar = 1} | Assert) &&
    (remove (extend {}) == {} | Assert) &&
    (extend (remove {foo = 2}) == {foo =1} | Assert) &&
    (let f | forall a b. {f: a -> a, arg: a ; b} -> a =
        fun rec => rec.f (rec.arg) in
      f { f = fun x => x ++ " suffix", arg = "foo" }
      == "foo suffix"
      | Assert)
  ),

  # records_dynamic_tail
  ({a = 1, b = "b"} | {a: Num, b: Str ; Dyn}) == {a = 1, b = "b"},
  ({a = 1, b = "b", c = false} | {a: Num, b: Str ; Dyn})
  == {a = 1, b = "b", c = false},
  ((fun r => r.b) | {a: Num ; Dyn} -> Dyn) {a = 1, b = 2} == 2,

  # records_open_contracts
  ({a = 0, b = 0} | {a | Num, ..}) == {a = 0, b = 0},
  let Contract = {a | Num} & {..} in
  ({a = 0, b = 0} | Contract) == {a = 0, b = 0},
  let Contract = {..} & {b | Num} in
  ({a = 0, b = 0} | Contract) == {a = 0, b = 0},
  let Contract = {a | Num, ..} & {b | Num, ..} in
  ({a = 0, b = 0, c = 0} | Contract) == {a = 0, b = 0, c = 0},

  # arrays
  ([1, "2", false] | Array Dyn) == [1, "2", false],
  ([1, 2, 3] | Array Num) == [1, 2, 3],
  (["1", "2", "false"] | Array Str) == ["1", "2", "false"],

  # full_annotations
  # Check that the contract introduced by the type annotation doesn't interact
  # with the `default` attribute
  ({foo : {bar: Bool} | default = {bar = false}} & {foo.bar = true}).foo.bar,

  # nested_metavalues
  # Regression test for #402
  let MyContract = { x | Str } in
  {
    foo | MyContract | default = { x = "From foo" },
    bar | {..} | default = foo
  } == { foo.x = "From foo", bar.x = "From foo"},

  # mixing type and record contracts
  let f | {foo | Num} -> {bar | Num} = fun r =>
    {bar = r.foo} in
  (f {foo = 1}).bar == 1,
  # user-written contract application
  let Extend = fun base label value =>
    let derived = if builtin.is_record base then
      (base & {foo | Num})
    else
      base in
    contract.apply derived label value in
  let Contract = Extend {bar | Num, ..} in
  let Id = Extend (fun label value => value) in
  ({bar = 1, foo = 1} | Contract)
    & ({baz = 1} | Id)
   == {foo = 1, bar = 1, baz = 1},
]
|> array.foldl (fun x y => (x | Assert) && y) true
