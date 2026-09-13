importScripts('pkg/qed.js');

const ready = wasm_bindgen({ module_or_path: 'pkg/qed_bg.wasm' });
const decoder = new TextDecoder();

onmessage = async (e) => {
  const { source, stdin } = e.data;
  try {
    await ready;
  } catch (err) {
    postMessage({ type: 'done', error: 'wasm init failed: ' + err });
    return;
  }
  let error = null;
  try {
    error = wasm_bindgen.execute(source, stdin, (bytes) => {
      postMessage({ type: 'out', text: decoder.decode(bytes) });
    });
  } catch (err) {
    error = String((err && err.message) || err);
  }
  postMessage({ type: 'done', error });
};
