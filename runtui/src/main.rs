use gpui::{
    div, rgb, App, AppContext, IntoElement, ParentElement, Render, SharedString, Styled,
    TitlebarOptions, ViewContext, VisualContext, WindowBounds, WindowOptions,
    Bounds
};

struct HelloWorld {
    text: SharedString,
}

impl Render for HelloWorld {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .bg(rgb(0x2e327d))
            .size_full()
            .justify_center()
            .items_center()
            .text_xl()
            .text_color(rgb(0xffffff))
            .child(format!("Hello, {}!", &self.text))
    }
}

fn main() {
    let win_options = WindowOptions {
        titlebar: Some(TitlebarOptions {
            title: "Hello, World!".into(),
            appears_transparent: true,
            traffic_light_position: Default::default(),
        }),
        bounds: WindowBounds::Fixed(Bounds::new(400.0, 400.0)),
        center: todo!(),
        focus: todo!(),
        show: todo!(),
        kind: todo!(),
        is_movable: todo!(),
        display_id: todo!(),
    };

    App::new().run(|cx: &mut AppContext| {
        cx.open_window(win_options, |cx| {
            cx.new_view(|_cx| HelloWorld {
                text: "World".into(),
            })
        });
    });
}
