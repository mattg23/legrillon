use std::str::FromStr;

use fltk::{
    app, button,
    enums::{self, Event},
    frame, group, input,
    prelude::*,
    text::{self, TextBuffer},
    window::{self, DoubleWindow},
};
use reqwest::Method;

use crate::{next_window_id, AppWindow, GlobalAppMsg, HasId};

pub struct RequestWindow {
    uri: String,
    headers: Vec<(String, String)>,
    method: reqwest::Method,
    body_buf: TextBuffer,
    global: app::Sender<GlobalAppMsg>,
    id: usize,
    window: DoubleWindow,
}

const UNIT: f64 = 1000.0;
const SUFFIX: [&str; 9] = ["B", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"];
pub fn human_bytes<T: Into<f64>>(bytes: T) -> String {
    let size = bytes.into();

    if size <= 0.0 {
        return "0 B".to_string();
    }

    let base = size.log10() / UNIT.log10();

    {
        let result = format!("{:.1}", UNIT.powf(base - base.floor()),)
            .trim_end_matches(".0")
            .to_owned();

        // Add suffix
        [&result, SUFFIX[base.floor() as usize]].join(" ")
    }
}

impl RequestWindow {
    pub fn new() -> Self {
        let id = next_window_id();

        let mut win = window::DoubleWindow::default()
            .with_size(1200, 800)
            .with_label("Le Grillon");

        let buf = text::TextBuffer::default();
        let result_buf = text::TextBuffer::default();

        let mut col = group::Flex::default_fill().column();
        let mut row = group::Flex::default().row();
        let mut verb_choice = fltk::menu::Choice::default();
        verb_choice.add_choice("GET");
        verb_choice.add_choice("POST");
        verb_choice.add_choice("PUT");
        verb_choice.add_choice("PATCH");
        verb_choice.add_choice("DELETE");
        verb_choice.set_value(0);

        row.fixed(&verb_choice, 196);

        let uri_label = frame::Frame::default().with_label("URI:");

        row.fixed(&uri_label, 64);
        let uri_input = input::Input::default();
        let mut runbtn = button::Button::default().with_label("🦗");
        runbtn.set_label_size(32);

        runbtn.set_compact(true);
        row.fixed(&runbtn, 64);
        row.end();
        col.fixed(&row, 32);
        let row = group::Flex::default_fill().row();
        let mut body = text::TextEditor::default();

        body.set_linenumber_width(12 * 3);

        body.set_buffer(buf.clone());
        body.set_text_font(enums::Font::Courier);
        let mut result = text::TextDisplay::default();

        result.set_linenumber_width(12 * 3);

        result.set_buffer(result_buf.clone());

        result.set_text_font(enums::Font::Courier);
        result.wrap_mode(text::WrapMode::AtBounds, 4);
        row.end();
        let mut status = frame::Frame::default();
        status.set_frame(enums::FrameType::FlatBox);
        col.fixed(&status, 32);
        col.end();

        win.end();
        win.make_resizable(true);
        win.show();
        let (s, _) = app::channel();
        let p_sender = s;
        win.handle(move |_, e| {
            if e == Event::Hide {
                println!("WINID={id}| {e:?}");
                p_sender.send(GlobalAppMsg::CloseWindow(id));
                return true;
            }

            false
        });

        let btn_ptr = runbtn.clone();
        let ptr_verb = verb_choice.clone();
        let ptr_result_text = result.clone();

        let mut buf2 = buf.clone();
        let buf3 = buf.clone();

        buf2.add_modify_callback(move |_, i, d, _, _| {
            if i > 0 || d > 0 {
                let mut body = body.clone();
                let buf = buf.clone();
                let lc = body.count_lines(0, buf.length(), false);
                let lc_width = ((f64::log10(lc as f64) as i64) + 1) * 12;
                let lc_width = lc_width.max(3 * 12);
                body.set_linenumber_width(lc_width as i32);
            }
        });

        let buf = buf3.clone();

        runbtn.set_callback(move |_| {
            let uri = uri_input.value();
            let body = buf.text();

            let verb =
                reqwest::Method::from_str(ptr_verb.choice().unwrap_or("GET".to_string()).as_str())
                    .unwrap_or(Method::GET);

            let mut result = result_buf.clone();
            let inner_btn_ptr = btn_ptr.clone();
            let mut inner_status_ptr = status.clone();
            let mut ptr_result_text = ptr_result_text.clone();
            btn_ptr.clone().deactivate();

            status.set_label(format!("Sending {verb} request...").as_str());

            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let start = std::time::Instant::now();
                match client.request(verb, uri).body(body).send().await {
                    Ok(resp) => {
                        // set result
                        let resp_time = std::time::Instant::now();
                        let mut cl = resp.content_length().unwrap_or(0);
                        let resp_status = resp.status();

                        match resp.text().await {
                            Ok(txt) => {
                                if cl == 0 {
                                    cl = txt.len() as u64;
                                }
                                let txt1 = txt.as_str();
                                let endpos = txt1.len();
                                result.set_text(txt1);

                                let lc = result.count_lines(0, endpos as i32) + 1;
                                let lc_width = ((f64::log10(lc as f64) as i64) + 1) * 12;
                                let lc_width = lc_width.max(3 * 12);
                                ptr_result_text.set_linenumber_width(lc_width as i32);
                            }
                            Err(e) => result.set_text(format!("{e:?}").as_str()),
                        };

                        let total_resp_time = std::time::Instant::now() - start;
                        let latency = resp_time - start;
                        let hbytes = human_bytes(cl as f64);
                        inner_status_ptr.set_label(
                            format!("STATUS={resp_status} | BYTES={hbytes} | RTT={total_resp_time:?} | LAT={latency:?}")
                                .as_str(),
                        );
                    }
                    Err(e) => {
                        inner_status_ptr.set_label("");
                        result.set_text(format!("{e:?}").as_str());
                    }
                }

                inner_btn_ptr.clone().activate();
                app::awake();
                app::redraw();
            });
        });

        Self {
            uri: String::new(),
            headers: vec![],
            method: reqwest::Method::GET,
            body_buf: buf3.clone(),
            global: s,
            id,
            window: win,
        }
    }
}

impl HasId for RequestWindow {
    fn id(&self) -> usize {
        self.id
    }
}

impl AppWindow for RequestWindow {
    fn close(&mut self) {
        self.window.hide();
    }
}
