mod app;
mod draw_utils;

use app::Application;
use core::ui::layout::style::Style;
use core::ui::component::Component;
use core::ui::component::text::Text;
use core::ui::component::button::Button;
use core::ui::component::container::VStack;
use std::sync::Arc;
use std::sync::Mutex;

fn main() {
    let button_text = Arc::new(Mutex::new("按钮".to_string()));
    
    Application::new().run(move |ctx| {
        VStack::new()
            .spacing(16.0)
            .child(Text::new("标题").font_size(240.0))
            .child(Button::dynamic(button_text.clone())
                    .font_size(160.0)
                    .on_click({
                let bt = button_text.clone();
                move || *bt.lock().unwrap() = "被点击了".to_string()
            }))
            .child(Text::new("你好，world！").font_size(360.0))
            .build(ctx)
    });
}