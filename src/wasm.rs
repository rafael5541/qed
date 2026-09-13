// wasm stuff

use std::io::Write;

use js_sys::Function;
use wasm_bindgen::prelude::*;

use crate::interp::Interpreter;
use crate::lexer::Lexer;
use crate::parser::Parser;

struct JsWriter {
    emit: Function,
    dead: bool,
}

impl JsWriter {
    fn new(emit: &Function) -> Self {
        JsWriter {
            emit: emit.clone(),
            dead: false,
        }
    }
}

impl Write for JsWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if self.dead {
            return Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "closed",
            ));
        }
        let arg: JsValue = js_sys::Uint8Array::from(bytes).into();
        match self.emit.call1(&JsValue::NULL, &arg) {
            Ok(_) => Ok(bytes.len()),
            Err(_) => {
                self.dead = true;
                Err(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    "closed",
                ))
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[wasm_bindgen]
pub fn execute(source: &str, stdin: &str, emit: &Function) -> Option<String> {
    let outcome = (|| {
        let tokens = Lexer::new(source).lex().map_err(|e| e.to_string())?;
        let program = Parser::new(tokens).parse().map_err(|e| e.to_string())?;
        Interpreter::run_streaming(&program, stdin, source, &mut JsWriter::new(emit))
            .map_err(|e| e.to_string())?;
        Ok::<(), String>(())
    })();
    outcome.err()
}
