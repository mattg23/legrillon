use std::{rc::Rc, str::FromStr};

use fltk::{
    app, button,
    enums::{self, Event},
    frame, group, input,
    prelude::*,
    text::{self},
    window::{self, DoubleWindow},
};
use reqwest::Method;

use crate::{db::OpenWindow, next_window_id, req_params::RequestParamsCtrl, AppWindow, GlobalAppMsg, HasId};

pub struct RequestWindow {
    uri: String,
    param_ctrl: Rc<RequestParamsCtrl>,
    method: reqwest::Method,
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
    pub fn new(wnd: Option<&OpenWindow>) -> Self {
        let id = wnd.map_or(next_window_id(), |f| f.id as usize);

        let mut win = window::DoubleWindow::default()
            .with_size(1200, 800)
            .with_label("Le Grillon");

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
        let mut uri_input = input::Input::default();
        let mut runbtn = button::Button::default().with_label("🦗");
        runbtn.set_label_size(32);

        runbtn.set_compact(true);
        row.fixed(&runbtn, 64);
        row.end();
        col.fixed(&row, 32);
        let row = group::Flex::default_fill().row();

        let mut req_params = RequestParamsCtrl::new();

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

        if let Some(wnd) = wnd{
            uri_input.set_value(wnd.uri.as_str());
            win.set_label(wnd.uri.as_str());
            if let Some(item) = verb_choice.find_item(wnd.method.as_str()){
                verb_choice.set_item(&item);
            }
            req_params.set(wnd);
        }

        


        let (s, _) = app::channel();
        let p_sender = s.clone();
        win.handle(move |_, e| {
            if e == Event::Hide {
                println!("HANDLE WINDOW EVENT| (RequestWindow) | WINID={id}| {e:?}");
                p_sender.send(GlobalAppMsg::CloseWindow(id));
                return true;
            }

            false
        });

        let btn_ptr = runbtn.clone();
        let ptr_verb = verb_choice.clone();
        let ptr_result_text = result.clone();

        let params_ptr = Rc::new(req_params);
        let params_ptr_run_cl = params_ptr.clone();

        let p_sender = s.clone();
        let p_win = win.clone();
        runbtn.set_callback(move |_| {
            let uri = uri_input.value();

            win.set_label(uri.clone().as_str());

            let body = params_ptr_run_cl.get_body();
            let headers = params_ptr_run_cl.get_headers();

            let verb =
                reqwest::Method::from_str(ptr_verb.choice().unwrap_or("GET".to_string()).as_str())
                    .unwrap_or(Method::GET);

            

            let mut result = result_buf.clone();
            let inner_btn_ptr = btn_ptr.clone();
            let mut inner_status_ptr = status.clone();
            let mut ptr_result_text = ptr_result_text.clone();
            btn_ptr.clone().deactivate();

            status.set_label(format!("Sending {verb} request...").as_str());

            let save_window = GlobalAppMsg::SaveWindowState(OpenWindow {
id: id as i32,
method: verb.to_string(),
uri: uri.clone(),
body: body.clone(),
path: "".to_string(),
query: "".to_string(),
headers: sqlx::types::Json(headers.iter().map(|f| (f.0.to_string(), f.1.to_str().unwrap().to_string())).collect())
            });

            p_sender.send(save_window);

            tokio::spawn(async move {
                let client = reqwest::Client::new();
                let start = std::time::Instant::now();                
                let req_builder = client.request(verb, uri).body(body).headers(headers);
                match req_builder.send().await {
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
            method: reqwest::Method::GET,
            global: s,
            id,
            window: p_win,
            param_ctrl: params_ptr,
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
