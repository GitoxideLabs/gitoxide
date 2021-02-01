use crate::file;
use bstr::{BStr, ByteSlice};
use dangerous::{BytesReader, Error};

fn read_config<'i, E>(r: &mut BytesReader<'i, E>) -> Result<Vec<file::Token>, E>
where
    E: Error<'i>,
{
    skip_whitespace_or_comment(r, ConsumeTo::NextToken);
    unimplemented!("sections and values");
}

enum ConsumeTo {
    NextToken,
    EndOfLine,
}

fn skip_whitespace_or_comment<'a, E>(r: &mut BytesReader<'a, E>, to_where: ConsumeTo) -> Option<&'a BStr> {
    fn skip_whitespace_or_comment<E>(r: &mut BytesReader<'_, E>, to_where: ConsumeTo) {
        fn skip_comment<E>(r: &mut BytesReader<'_, E>) -> usize {
            if r.peek_eq(b'#') {
                r.take_while(|c| c != b'\n').len()
            } else {
                0
            }
        }

        let (mut last, mut current) = (0, 0);
        loop {
            current += skip_comment(r);
            current += r
                .take_while(|c| {
                    let iwb = c.is_ascii_whitespace();
                    iwb && match to_where {
                        ConsumeTo::NextToken => true,
                        ConsumeTo::EndOfLine => c != b'\n',
                    }
                })
                .len();
            if last == current {
                break;
            }
            last = current;
        }
    }
    let parsed = r
        .take_consumed(|r| skip_whitespace_or_comment(r, to_where))
        .as_dangerous();
    if parsed.is_empty() {
        None
    } else {
        Some(parsed.as_bstr())
    }
}

#[cfg(test)]
mod tests {
    mod comments {
        use crate::parse::{skip_whitespace_or_comment, ConsumeTo};
        use bstr::ByteSlice;
        use dangerous::Input;

        #[test]
        fn whitespace_skipping_whitespace() {
            let i = b"     \n     \t ";
            let (res, remaining) =
                dangerous::input(i).read_infallible(|r| skip_whitespace_or_comment(r, ConsumeTo::NextToken));
            assert!(remaining.is_empty());
            assert_eq!(res, Some(i.as_bstr()));
        }
    }
}
