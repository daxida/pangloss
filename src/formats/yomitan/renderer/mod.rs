// Maybe use maud (?)

mod term_bank;
mod term_meta_bank;

trait Renderer {
    fn render_into(&self, out: &mut String);

    fn render(&self) -> String {
        let mut out = String::new();
        self.render_into(&mut out);
        out
    }
}

impl<T: Renderer> Renderer for Option<T> {
    fn render_into(&self, out: &mut String) {
        if let Some(inner) = self {
            inner.render_into(out);
        }
    }
}
