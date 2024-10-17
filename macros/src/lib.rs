use std::path::Path;

use {
    proc_macro::TokenStream,
    proc_macro_error::{abort_call_site, proc_macro_error},
    quote::quote,
    syn::{parse_macro_input, Expr, ExprLit, ItemEnum, Lit, LitStr},
};

#[proc_macro_error]
#[proc_macro_attribute]
pub fn generate_wgsl_enum(args: TokenStream, item: TokenStream) -> TokenStream {
    let path = parse_macro_input!(args as Option<LitStr>).map(|lit| lit.value());

    let Some(path) = path.as_ref().map(Path::new) else {
        abort_call_site!("Must provide a path to generate the enum at");
    };

    let parsed_item = parse_macro_input!(item as ItemEnum);

    let mut counter = None;
    let content = parsed_item
        .variants
        .iter()
        .map(|variant| {
            if let Some((
                _,
                Expr::Lit(ExprLit {
                    lit: Lit::Int(ref discriminant),
                    ..
                }),
            )) = variant.discriminant
            {
                counter = Some(discriminant.base10_parse().unwrap());
            } else {
                counter = Some(counter.map_or(0, |c| c + 1));
            }

            format!(
                "const {}: u32 = {};",
                variant.ident.to_string(),
                counter.unwrap()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    std::fs::write(path, content).expect("failed to write .wgsl");

    quote! {
        #parsed_item
    }
    .into()
}
