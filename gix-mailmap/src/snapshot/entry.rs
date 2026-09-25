use bstr::BString;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct NameEntry {
    pub(crate) new_name: Option<BString>,
    pub(crate) new_email: Option<BString>,
    pub(crate) old_name: BString,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) struct EmailEntry {
    pub(crate) new_name: Option<BString>,
    pub(crate) new_email: Option<BString>,
    pub(crate) old_email: BString,

    pub(crate) entries_by_old_name: Vec<NameEntry>,
}

impl EmailEntry {
    pub fn merge(&mut self, later: &mut Self) {
        if later.new_name.is_some() {
            self.new_name = later.new_name.take();
        }
        if later.new_email.is_some() {
            self.new_email = later.new_email.take();
        }
        self.entries_by_old_name.append(&mut later.entries_by_old_name);
    }
}

impl<'a> From<crate::Entry<'a>> for EmailEntry {
    fn from(
        crate::Entry {
            new_name,
            new_email,
            old_name,
            old_email,
        }: crate::Entry<'a>,
    ) -> Self {
        let mut new_name = new_name.map(ToOwned::to_owned);
        let mut new_email = new_email.map(ToOwned::to_owned);
        let entries_by_old_name = old_name
            .map(|name| {
                vec![NameEntry {
                    new_name: new_name.take(),
                    new_email: new_email.take(),
                    old_name: name.into(),
                }]
            })
            .unwrap_or_default();
        EmailEntry {
            new_name,
            new_email,
            old_email: old_email.into(),
            entries_by_old_name,
        }
    }
}
