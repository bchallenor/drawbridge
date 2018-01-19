use rusoto_ec2::Tag;

pub trait TagFinder<'a> {
    fn find_tag(self, key: &str) -> Option<&'a str>;
}

impl<'a, T> TagFinder<'a> for T
where
    T: IntoIterator<Item = &'a Tag>,
{
    fn find_tag(self, key: &str) -> Option<&'a str> {
        self.into_iter()
            .filter_map(|tag| match *tag {
                Tag {
                    key: Some(ref k),
                    value: Some(ref v),
                } if k == key =>
                {
                    Some(v as &str)
                }
                _ => None,
            })
            .next()
    }
}
