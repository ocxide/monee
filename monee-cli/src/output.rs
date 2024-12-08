use std::io::stdout;

pub struct Listter<I> {
    iter: I,
    any_written: bool,
}

impl<I> Listter<I> {
    pub fn new(iter: I) -> Listter<I> {
        Listter {
            iter,
            any_written: false,
        }
    }
}

impl<I> Iterator for Listter<I>
where
    I: Iterator,
    I::Item: std::fmt::Display,
{
    type Item = ListterItem<I::Item>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(next) = self.iter.next() {
            self.any_written = true;
            return Some(ListterItem::Item(next));
        }

        if self.any_written {
            None
        } else {
            self.any_written = true;
            Some(ListterItem::Empty)
        }
    }
}

pub enum ListterItem<T> {
    Empty,
    Item(T),
}

impl<T> std::fmt::Display for ListterItem<T>
where
    T: std::fmt::Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ListterItem::Empty => write!(f, "<None>"),
            ListterItem::Item(item) => write!(f, "{}", item),
        }
    }
}

pub fn print_data(data: impl ExactSizeIterator<Item = impl std::fmt::Display>) {
    let listter = Listter::new(data);
    print_iter(listter);
}

pub fn print_iter(iter: impl Iterator<Item = impl std::fmt::Display>) {
    use std::io::Write;
    let mut stdout = stdout();
    for item in iter {
        let result = writeln!(&mut stdout, "{}", item);
        result.unwrap();
    }
}

pub struct Formatted<F>(pub F)
where
    F: Fn(&'_ mut std::fmt::Formatter<'_>) -> std::fmt::Result;

impl<F> std::fmt::Display for Formatted<F>
where
    F: Fn(&'_ mut std::fmt::Formatter<'_>) -> std::fmt::Result,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self.0)(f)
    }
}

#[macro_export]
macro_rules! formatted {(
    $($args:tt)+
) => ($crate::output::Formatted(
    move |fmt: &'_ mut ::core::fmt::Formatter<'_>| {
        write!(fmt, $($args)+)
    }
))}

pub use formatted;
