mod view;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use anyhow::Context as _;
use gpui_kit::{AppContext as _, TitlebarOptions, WindowOptions};

use self::view::DiffView;

/// Opens the side-by-side window and blocks until it is closed.
pub(crate) fn run(left: &Path, right: &Path) -> anyhow::Result<()> {
    let (left, right) = (left.to_owned(), right.to_owned());
    // `run` needs a 'static closure, so a failure to open travels back through a cell.
    let open_error = Rc::new(RefCell::new(None));
    let open_error_slot = Rc::clone(&open_error);

    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        // Linux and Windows keep running without windows; diffly is done when its window is.
        cx.on_window_closed(|cx, _| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let options = WindowOptions {
            titlebar: Some(TitlebarOptions {
                title: Some(
                    format!(
                        "diffly \u{2014} {} \u{2194} {}",
                        left.display(),
                        right.display()
                    )
                    .into(),
                ),
                ..Default::default()
            }),
            ..Default::default()
        };
        let opened = gpui_kit::open_window(options, cx, |_, cx| {
            cx.new(|_| DiffView::load(&left, &right))
        });
        if let Err(err) = opened {
            *open_error_slot.borrow_mut() = Some(err);
            cx.quit();
        }
    });

    match open_error.take() {
        Some(err) => Err(err).context("opening the diffly window"),
        None => Ok(()),
    }
}
