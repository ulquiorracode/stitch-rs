use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Token, Type, Visibility};

/// Syntax arguments for `define_pipeline!`:
/// ```rust,ignore
/// define_pipeline! {
///     name: Chat,
///     context: ChatContext<'_>,
///     intent: ChatIntent<'_>,
///     outcome: (),
///     error: (),
///     vis: pub,
/// }
/// ```
pub struct PipelineDef {
    pub vis: Visibility,
    pub name: Ident,
    pub context_ty: Type,
    pub intent_ty: Type,
    pub outcome_ty: Type,
    pub error_ty: Type,
}

impl Parse for PipelineDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut vis = Visibility::Inherited;
        let mut name = None;
        let mut context_ty = None;
        let mut intent_ty = None;
        let mut outcome_ty = None;
        let mut error_ty = None;

        while !input.is_empty() {
            let key: Ident = input.parse()?;
            input.parse::<Token![:]>()?;

            match key.to_string().as_str() {
                "vis" => {
                    vis = input.parse()?;
                }
                "name" => {
                    name = Some(input.parse()?);
                }
                "context" => {
                    context_ty = Some(input.parse()?);
                }
                "intent" => {
                    intent_ty = Some(input.parse()?);
                }
                "outcome" => {
                    outcome_ty = Some(input.parse()?);
                }
                "error" => {
                    error_ty = Some(input.parse()?);
                }
                other => {
                    return Err(syn::Error::new(
                        key.span(),
                        format!(
                            "unknown key `{other}`, expected `name`, `context`, `intent`, `outcome`, `error`, or `vis`"
                        ),
                    ));
                }
            }

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }

        let name = name
            .ok_or_else(|| syn::Error::new(input.span(), "missing `name` in define_pipeline!"))?;
        let context_ty = context_ty.ok_or_else(|| {
            syn::Error::new(input.span(), "missing `context` in define_pipeline!")
        })?;
        let intent_ty = intent_ty
            .ok_or_else(|| syn::Error::new(input.span(), "missing `intent` in define_pipeline!"))?;
        let outcome_ty = outcome_ty.unwrap_or_else(|| syn::parse_quote!(()));
        let error_ty = error_ty.unwrap_or_else(|| syn::parse_quote!(()));

        Ok(PipelineDef {
            vis,
            name,
            context_ty,
            intent_ty,
            outcome_ty,
            error_ty,
        })
    }
}

pub fn expand_define_pipeline(def: PipelineDef) -> TokenStream2 {
    let PipelineDef {
        vis,
        name,
        context_ty,
        intent_ty,
        outcome_ty,
        error_ty,
    } = def;

    let layer_trait = quote::format_ident!("{}Layer", name);
    let terminal_struct = quote::format_ident!("{}Terminal", name);
    let pipeline_struct = quote::format_ident!("{}Pipeline", name);

    quote! {
        /// Generated SMA Layer trait for intercepting, auditing, or transforming U-cycle execution.
        #vis trait #layer_trait: ::core::marker::Send + ::core::marker::Sync {
            /// Descent phase: inspection, security validation, or short-circuiting.
            fn on_enter(
                &self,
                ctx: &mut #context_ty,
                intent: &#intent_ty,
            ) -> ::stitch_core::FlowControl<#outcome_ty, #error_ty, ()>;

            /// Ascent phase: audit logging, telemetry, or output transformation.
            fn on_exit(
                &self,
                ctx: &mut #context_ty,
                intent: &#intent_ty,
                outcome: &mut ::core::result::Result<#outcome_ty, #error_ty>,
            );
        }

        /// Terminal leaf node terminating U-cycle descent.
        #[derive(Debug, Clone, Copy, Default)]
        #vis struct #terminal_struct;

        impl ::stitch_core::middleware::Terminal<#context_ty, #intent_ty, #outcome_ty, #error_ty>
            for #terminal_struct
        {
            #[inline(always)]
            fn execute(
                &mut self,
                _ctx: &mut #context_ty,
                _intent: #intent_ty,
            ) -> ::core::result::Result<#outcome_ty, #error_ty> {
                ::core::result::Result::Ok(Default::default())
            }
        }

        /// Dynamic composite SMA pipeline orchestrator.
        #[derive(Clone)]
        #vis struct #pipeline_struct {
            layers: ::std::vec::Vec<::std::sync::Arc<dyn #layer_trait>>,
        }

        impl ::core::default::Default for #pipeline_struct {
            fn default() -> Self {
                Self::new()
            }
        }

        impl #pipeline_struct {
            /// Creates a new empty pipeline.
            #[inline]
            pub fn new() -> Self {
                Self {
                    layers: ::std::vec::Vec::new(),
                }
            }

            /// Appends a middleware layer to this pipeline.
            pub fn with_layer(mut self, layer: ::std::sync::Arc<dyn #layer_trait>) -> Self {
                self.layers.push(layer);
                self
            }

            /// Executes the complete U-cycle traversal across all layers.
            #[allow(clippy::result_unit_err)]
            pub fn dispatch(
                &self,
                ctx: &mut #context_ty,
                intent: &#intent_ty,
            ) -> ::core::result::Result<#outcome_ty, #error_ty> {
                let mut halted = false;
                let mut entered_count = 0;
                let mut early_outcome: ::core::option::Option<#outcome_ty> = ::core::option::Option::None;
                let mut early_err: ::core::option::Option<#error_ty> = ::core::option::Option::None;

                for layer in &self.layers {
                    match layer.on_enter(ctx, intent) {
                        ::stitch_core::FlowControl::Proceed(()) => {
                            entered_count += 1;
                        }
                        ::stitch_core::FlowControl::ShortCircuit(out) => {
                            entered_count += 1;
                            early_outcome = ::core::option::Option::Some(out);
                            break;
                        }
                        ::stitch_core::FlowControl::Halt(err) => {
                            halted = true;
                            early_err = ::core::option::Option::Some(err);
                            break;
                        }
                    }
                }

                let mut outcome = if halted {
                    ::core::result::Result::Err(early_err.unwrap_or_default())
                } else if let ::core::option::Option::Some(out) = early_outcome {
                    ::core::result::Result::Ok(out)
                } else {
                    let mut terminal = #terminal_struct;
                    use ::stitch_core::middleware::Terminal;
                    terminal.execute(ctx, intent.clone())
                };

                for layer in self.layers.iter().take(entered_count).rev() {
                    layer.on_exit(ctx, intent, &mut outcome);
                }

                outcome
            }
        }
    }
}
