use std::error::Error as StdError;

pub fn inspect<E>(err: E) -> Vec<String>
where
    E: Into<Box<dyn StdError + Send + Sync>>,
{
    let berr = err.into();
    let mut err = Some(&*berr as &(dyn StdError + 'static));
    let mut errs = Vec::new();
    while let Some(e) = err {
        errs.push(e.to_string());
        err = e.source();
    }
    errs
}
