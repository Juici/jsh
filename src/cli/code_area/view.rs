use std::sync::Arc;

use super::{CodeArea, CodeBuffer, PendingCode};

use crate::cli::term::buffer::BufferBuilder;
use crate::cli::term::utils::wcswidth;
use crate::cli::ui::Text;

pub struct View {
    prompt: Arc<Text>,
    rprompt: Option<Arc<Text>>,
    code: Text,
    dot: usize,
    // TODO: Errors.
}

impl View {
    pub async fn get(code_area: &CodeArea) -> View {
        let state = code_area.clone_state().await;

        let mut code = state.buffer;
        let (_from, _to) = patch_pending(&mut code, &state.pending);

        // TODO: Highlighter.
        let styled_code = Text::plain(code.content);

        // TODO: Prompts.
        let prompt = code_area.prompt.prompt().await;

        let rprompt = if state.hide_rprompt {
            None
        } else {
            Some(code_area.rprompt.prompt().await)
        };

        View {
            prompt,
            rprompt,
            code: styled_code,
            dot: code.dot,
        }
    }

    pub fn render_view(self, buf: &mut BufferBuilder) {
        buf.eager_wrap = true;

        buf.write_text(&self.prompt);
        if buf.lines.len() == 1 && buf.col * 2 < buf.width {
            buf.indent = buf.col;
        }

        // TODO: Optimize and reduce allocations.
        let parts = self.code.split_at(self.dot);
        buf.write_text(&parts.0).dot().write_text(&parts.1);

        buf.eager_wrap = false;
        buf.indent = 0;

        if let Some(rprompt) = self.rprompt {
            let rprompt_width = rprompt
                .iter()
                .map(|seg| wcswidth(&seg.text))
                .fold(0u16, std::ops::Add::add);

            if rprompt_width > 0 {
                // Don't write rprompt if there is not room.
                match buf
                    .width
                    .checked_sub(buf.col)
                    .and_then(|d| d.checked_sub(rprompt_width))
                {
                    Some(0) | None => {}
                    Some(pad) => {
                        buf.write_spaces(pad as usize);
                        buf.write_text(&rprompt);
                    }
                }
            }
        }

        // TODO: Render errors.
    }
}

fn patch_pending(b: &mut CodeBuffer, p: &PendingCode) -> (usize, usize) {
    if p.from > p.to || p.to > b.content.len() {
        return (0, 0); // Invalid.
    }

    if p.from == p.to && p.content.is_empty() {
        return (0, 0);
    }

    b.content.replace_range(p.from..p.to, &p.content);
    b.dot = match b.dot {
        // Before the replaced region, leave it.
        dot if dot < p.from => dot,
        // Within the replaced region, move to end.
        dot if dot >= p.from && dot < p.to => p.from + p.content.len(),
        // After the replaced region, maintain relative position.
        dot if dot >= p.to => dot - (p.to - p.from) + p.content.len(),
        dot => dot,
    };

    (p.from, p.from + p.content.len())
}
