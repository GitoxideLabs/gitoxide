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
                if let Some(move_token) =
                    tokens.next_if(|token| matches!(token, TokenTree::Ident(ident) if ident == "move"))
                    && matches!(tokens.peek(), Some(TokenTree::Punct(punct)) if punct.as_char() == '|')
                {
                    output.extend([move_token]);
                }
            }
            TokenTree::Punct(punct)
                if punct.as_char() == '.'
                    && tokens
                        .next_if(|token| matches!(token, TokenTree::Ident(ident) if ident == "await"))
                        .is_some() => {}
            token => output.extend([token]),
        }
    }
    output
}
