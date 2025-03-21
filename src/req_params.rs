use std::str::FromStr;

use fltk::{
    enums::{self},
    group::{Flex, Tabs},
    prelude::*,
    text::{self, TextBuffer},
};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

pub struct RequestParamsCtrl {
    headers_buf: TextBuffer,
    body_buf: TextBuffer,
}

impl RequestParamsCtrl {
    pub fn new() -> Self {
        let mut tab = Tabs::default_fill();

        let grp1 = Flex::default_fill().with_label("Body\t\t").row();

        let body_buf = text::TextBuffer::default();
        let mut body = text::TextEditor::default();

        body.set_linenumber_width(12 * 3);

        body.set_buffer(body_buf.clone());
        body.set_text_font(enums::Font::Courier);
        grp1.end();

        let grp2 = Flex::default_fill().with_label("Headers\t\t").row();
        let headers_buf = text::TextBuffer::default();
        let mut headers = text::TextEditor::default();

        headers.set_linenumber_width(12 * 3);
        headers.set_buffer(headers_buf.clone());
        headers.set_text_font(enums::Font::Courier);

        grp2.end();
        tab.end();
        tab.auto_layout();

        // setup auto expand for line numbers
        let buf = body_buf.clone();
        body_buf.clone().add_modify_callback(move |_, i, d, _, _| {
            if i > 0 || d > 0 {
                let mut body = body.clone();
                let buf = buf.clone();
                let lc = body.count_lines(0, buf.length(), false);
                let lc_width = ((f64::log10(lc as f64) as i64) + 1) * 12;
                let lc_width = lc_width.max(3 * 12);
                body.set_linenumber_width(lc_width as i32);
            }
        });
        let buf = headers_buf.clone();
        headers_buf
            .clone()
            .add_modify_callback(move |_, i, d, _, _| {
                if i > 0 || d > 0 {
                    let mut body = headers.clone();
                    let buf = buf.clone();
                    let lc = body.count_lines(0, buf.length(), false);
                    let lc_width = ((f64::log10(lc as f64) as i64) + 1) * 12;
                    let lc_width = lc_width.max(3 * 12);
                    body.set_linenumber_width(lc_width as i32);
                }
            });

        Self {
            headers_buf,
            body_buf,
        }
    }

    pub fn get_body(&self) -> String {
        self.body_buf.text()
    }

    pub fn get_headers(&self) -> HeaderMap {
        let possibles = self
            .headers_buf
            .text()
            .lines()
            .filter_map(|l| l.split_once(':'))
            .map(|(n, v)| (HeaderName::from_str(n), HeaderValue::from_str(v)))
            .filter(|(n, v)| n.is_ok() && v.is_ok())
            .map(|(n, v)| (n.unwrap(), v.unwrap()))
            .collect::<HeaderMap>();

        possibles
    }

    pub fn set(&mut self, wnd: &crate::db::OpenWindow) {
        self.body_buf.set_text(&wnd.body);

        for (n, v) in wnd.headers.0.iter() {
            self.headers_buf.append(format!("{v}: {n} \n").as_str());
        }
    }
}
