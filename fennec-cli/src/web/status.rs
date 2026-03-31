use maud::Render;

#[derive(Copy, Clone)]
pub enum Status {
    Ok,
    Error,
}

impl Render for Status {
    fn render_to(&self, buffer: &mut String) {
        match self {
            Self::Ok => buffer.push_str("is-success"),
            Self::Error => buffer.push_str("is-danger"),
        }
    }
}
