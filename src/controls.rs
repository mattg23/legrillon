use fltk::{
    app::Sender,
    button,
    enums::Event,
    group, image,
    prelude::*,
    window::{self, SingleWindow},
};

use crate::{AppWindow, GlobalAppMsg, HasId, next_window_id};

pub struct MainControls {
    global_msg_sender: Sender<GlobalAppMsg>,
    window_ptr: Option<SingleWindow>,
    id: usize,
}

impl MainControls {
    pub fn new(global_msg_sender: Sender<GlobalAppMsg>) -> Self {
        let mut self_ = MainControls {
            global_msg_sender,
            window_ptr: None,
            id: next_window_id(),
        };
        self_.setup();
        self_
    }

    fn setup(&mut self) {
        let mut ctrl_window = window::SingleWindow::default()
            .with_size(600, 64)
            .with_label("Le Grillon");

        let row = group::Flex::default_fill().row();

        let mut new_req_window_button = button::Button::default().with_label("New 🦗");
        new_req_window_button.set_label_size(32);
        let p_sender = self.global_msg_sender.clone();
        new_req_window_button.set_callback(move |_| {
            p_sender.send(GlobalAppMsg::OpenEmptyWindow);
        });
        row.end();

        ctrl_window.end();
        ctrl_window.show();

        let loadimg = image::JpegImage::load("./assets/legrillon.jpg");
        if let Ok(image) = loadimg {
            ctrl_window.set_icon(Some(image));
        } else {
            print!("IMGERR= {loadimg:?}");
        }
        let p_sender = self.global_msg_sender.clone();
        ctrl_window.handle(move |_, e| {
            if e == Event::Hide {
                p_sender.send(GlobalAppMsg::CloseApp);
                true
            } else {
                false
            }
        });

        self.window_ptr = Some(ctrl_window);
    }
}

impl HasId for MainControls {
    fn id(&self) -> usize {
        self.id
    }
}

impl AppWindow for MainControls {
    fn close(&mut self) {
        self.window_ptr.as_mut().unwrap().hide();
    }
}
