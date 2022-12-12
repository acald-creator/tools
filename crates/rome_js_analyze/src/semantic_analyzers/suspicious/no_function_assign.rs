use crate::semantic_services::Semantic;
use rome_analyze::{context::RuleContext, declare_rule, Rule, RuleDiagnostic};
use rome_console::markup;
use rome_js_semantic::{Reference, ReferencesExtensions};
use rome_js_syntax::{JsFunctionDeclaration, JsIdentifierBinding};
use rome_rowan::AstNode;

declare_rule! {
    /// Disallow reassigning function declarations.
    ///
    /// ## Examples
    ///
    /// ### Invalid
    ///
    /// ```js,expect_diagnostic
    /// function foo() { };
    /// foo = bar;
    /// ```
    ///
    /// ```js,expect_diagnostic
    /// function foo() {
    ///     foo = bar;
    ///  }
    /// ```
    ///
    /// ```js,expect_diagnostic
    /// foo = bar;
    /// function foo() { };
    /// ```
    ///
    /// ```js,expect_diagnostic
    /// [foo] = bar;
    /// function foo() { };
    /// ```
    ///
    /// ```js,expect_diagnostic
    /// ({ x: foo = 0 } = bar);
    /// function foo() { };
    /// ```
    ///
    /// ```js,expect_diagnostic
    /// function foo() {
    ///     [foo] = bar;
    ///  }
    /// ```
    /// ```js,expect_diagnostic
    /// (function () {
    ///     ({ x: foo = 0 } = bar);
    ///     function foo() { };
    ///  })();
    /// ```
    ///
    /// ## Valid
    ///
    /// ```js
    /// function foo() {
    ///     var foo = bar;
    ///  }
    /// ```
    ///
    /// ```js
    /// function foo(foo) {
    ///     foo = bar;
    ///  }
    /// ```
    ///
    /// ```js
    /// function foo() {
    ///     var foo;
    ///     foo = bar;
    ///  }
    /// ```
    ///
    /// ```js
    /// var foo = () => {};
    /// foo = bar;
    /// ```
    ///
    /// ```js
    /// var foo = function() {};
    /// foo = bar;
    /// ```
    ///
    /// ```js
    /// var foo = function() {
    ///     foo = bar;
    ///  };
    /// ```
    ///
    /// ```js
    /// import bar from 'bar';
    /// function foo() {
    ///     var foo = bar;
    /// }
    /// ```
    pub(crate) NoFunctionAssign {
        version: "0.7.0",
        name: "noFunctionAssign",
        recommended: true,
    }
}

pub struct State {
    id: JsIdentifierBinding,
    all_writes: Vec<Reference>,
}

impl Rule for NoFunctionAssign {
    type Query = Semantic<JsFunctionDeclaration>;
    type State = State;
    type Signals = Option<Self::State>;
    type Options = ();

    fn run(ctx: &RuleContext<Self>) -> Option<Self::State> {
        let declaration = ctx.query();
        let model = ctx.model();

        let id = declaration.id().ok()?;
        let id = id.as_js_identifier_binding()?;
        let all_writes: Vec<Reference> = id.all_writes(model).collect();

        if all_writes.is_empty() {
            None
        } else {
            Some(State {
                id: id.clone(),
                all_writes,
            })
        }
    }

    fn diagnostic(_: &RuleContext<Self>, state: &Self::State) -> Option<RuleDiagnostic> {
        let mut diag = RuleDiagnostic::new(
            rule_category!(),
            state.id.syntax().text_trimmed_range(),
            markup! {
                "Do not reassign a function declaration."
            },
        );

        let mut hoisted_quantity = 0;
        for reference in state.all_writes.iter() {
            let node = reference.syntax();
            diag = diag.detail(node.text_trimmed_range(), "Reassigned here.");

            hoisted_quantity += i32::from(reference.is_using_hoisted_declaration());
        }

        let diag = if hoisted_quantity > 0 {
            diag.note(
                markup! {"Reassignment happens here because the function declaration is hoisted."},
            )
        } else {
            diag
        };

        let diag = diag.note(markup! {"Use a local variable instead."});

        Some(diag)
    }
}