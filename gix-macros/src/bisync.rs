use proc_macro2::{Group, TokenStream, TokenTree};

pub(crate) fn sync(input: TokenStream) -> TokenStream {
    let mut output = TokenStream::new();
    let mut tokens = input.into_iter().peekable();

    while let Some(token) = tokens.next() {
        match token {
            TokenTree::Group(group) => {
                let mut replacement = Group::new(group.delimiter(), sync(group.stream()));
                replacement.set_span(group.span());
                output.extend([TokenTree::Group(replacement)]);
            }
            TokenTree::Ident(ident) if ident == "async" => {
                if matches!(tokens.peek(), Some(TokenTree::Ident(ident)) if ident == "move") {
                    let move_token = tokens.next();
                    if matches!(tokens.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == '|') {
                        output.extend(move_token);
                    }
                }
            }
            TokenTree::Punct(punct)
                if punct.as_char() == '.'
                    && matches!(tokens.peek(), Some(TokenTree::Ident(ident)) if ident == "await") =>
            {
                tokens.next();
            }
            token => output.extend([token]),
        }
    }
    output
}
