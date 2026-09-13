const editor = document.getElementById('editor');
const output = document.getElementById('output');
const runBtn = document.getElementById('run');
const stopBtn = document.getElementById('stop');
const statusEl = document.getElementById('status');
const stdinEl = document.getElementById('stdin');
const exampleSel = document.getElementById('examples');
const keysEl = document.getElementById('keys');

const SUB = {0:'₀',1:'₁',2:'₂',3:'₃',4:'₄',5:'₅',6:'₆',7:'₇',8:'₈',9:'₉','+':'₊','-':'₋','=':'₌','(':'₍',')':'₎',a:'ₐ',e:'ₑ',h:'ₕ',i:'ᵢ',j:'ⱼ',k:'ₖ',l:'ₗ',m:'ₘ',n:'ₙ',o:'ₒ',p:'ₚ',r:'ᵣ',s:'ₛ',t:'ₜ',u:'ᵤ',v:'ᵥ',x:'ₓ'};
const SUP = {0:'⁰',1:'¹',2:'²',3:'³',4:'⁴',5:'⁵',6:'⁶',7:'⁷',8:'⁸',9:'⁹','+':'⁺','-':'⁻','=':'⁼','(':'⁽',')':'⁾',a:'ᵃ',b:'ᵇ',c:'ᶜ',d:'ᵈ',e:'ᵉ',f:'ᶠ',g:'ᵍ',h:'ʰ',i:'ⁱ',j:'ʲ',k:'ᵏ',l:'ˡ',m:'ᵐ',n:'ⁿ',o:'ᵒ',p:'ᵖ',r:'ʳ',s:'ˢ',t:'ᵗ',u:'ᵘ',v:'ᵛ',w:'ʷ',x:'ˣ',y:'ʸ',z:'ᶻ'};

let statusTimer = null;
function flash(msg) {
  statusEl.textContent = msg;
  clearTimeout(statusTimer);
  statusTimer = setTimeout(() => {
    if (statusEl.textContent === msg && !runBtn.disabled) statusEl.textContent = '';
  }, 1500);
}

const GROUPS = [
  ['sym', ['∴', '∎', 'ℳ', '⊢', '↓', '⊨', 'χ', 'δ', 'ε', '⊕', '⨁', '𝒞', '𝒩', '𝒮', 'μ', '∧', '∨', '¬', '⊤', '⊥', '∀', '∃', '∈', '∤', '∣', '|', '≠', '≤', '≥', '√', '∑', '∏', '∞', '⌊', '⌋', '⌈', '⌉', '…', '!', '∪', '∩', '⊆', '∘', '≡']],
  ['sub', ['₀', '₁', '₂', '₃', '₄', '₅', '₆', '₇', '₈', '₉', '₊', '₋', '₌', '₍', '₎', 'ₐ', 'ₑ', 'ₕ', 'ᵢ', 'ⱼ', 'ₖ', 'ₗ', 'ₘ', 'ₙ', 'ₒ', 'ₚ', 'ᵣ', 'ₛ', 'ₜ', 'ᵤ', 'ᵥ', 'ₓ']],
  ['sup', ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹', '⁺', '⁻', '⁼', '⁽', '⁾', 'ᵃ', 'ᵇ', 'ᶜ', 'ᵈ', 'ᵉ', 'ᶠ', 'ᵍ', 'ʰ', 'ⁱ', 'ʲ', 'ᵏ', 'ˡ', 'ᵐ', 'ⁿ', 'ᵒ', 'ᵖ', 'ʳ', 'ˢ', 'ᵗ', 'ᵘ', 'ᵛ', 'ʷ', 'ˣ', 'ʸ', 'ᶻ']],
];

for (const [name, syms] of GROUPS) {
  const tag = document.createElement('span');
  tag.className = 'group';
  tag.textContent = name;
  keysEl.appendChild(tag);
  for (const s of syms) {
    const b = document.createElement('button');
    b.textContent = s;
    b.title = s;
    b.onclick = () => insertSymbol(s);
    keysEl.appendChild(b);
  }
  keysEl.appendChild(document.createElement('br'));
}

function insertSymbol(s) {
  const a = editor.selectionStart ?? editor.value.length;
  const b = editor.selectionEnd ?? editor.value.length;
  editor.value = editor.value.slice(0, a) + s + editor.value.slice(b);
  editor.selectionStart = editor.selectionEnd = a + s.length;
  editor.focus();
}

const ALIAS = {chi:'χ',dec:'δ',eps:'ε',bigoplus:'⨁',prod:'∏',source:'𝒮',C:'𝒞',N:'𝒩',mu:'μ',or:'∨',true:'⊤',false:'⊥',forall:'∀',exists:'∃',in:'∈',divides:'∣',notdiv:'∤',inf:'∞',union:'∪',inter:'∩',subset:'⊆',compose:'∘',halts:'↓',meta:'ℳ'};

editor.addEventListener('keydown', (e) => {
  if (e.key === 'Tab') {
    e.preventDefault();
    insertSymbol('  ');
    return;
  }
  if (!e.altKey || e.ctrlKey || e.metaKey) return;
  if (e.key === 'Enter') {
    e.preventDefault();
    const a = editor.selectionStart ?? editor.value.length;
    const b = editor.selectionEnd ?? editor.value.length;
    const m = /[A-Za-z]+$/.exec(editor.value.slice(0, a));
    if (!m) {
      flash('nothing to convert');
      return;
    }
    const s = ALIAS[m[0]];
    if (s === undefined) {
      flash('no symbol for ' + m[0]);
      return;
    }
    editor.value = editor.value.slice(0, a - m[0].length) + s + editor.value.slice(b);
    editor.selectionStart = editor.selectionEnd = a - m[0].length + s.length;
    editor.focus();
    return;
  }
  if (!e.altKey || e.ctrlKey || e.metaKey) return;
  let k = null;
  const mDigit = /^(Digit|Numpad)([0-9])$/.exec(e.code || '');
  const mKey = /^Key([A-Z])$/.exec(e.code || '');
  if (mDigit) k = mDigit[2];
  else if (mKey) k = mKey[1].toLowerCase();
  else if (e.key.length === 1) k = e.key.toLowerCase();
  else return;
  e.preventDefault();
  const map = e.shiftKey ? SUP : SUB;
  const s = map[k];
  if (s !== undefined) {
    insertSymbol(s);
  } else {
    flash('no ' + (e.shiftKey ? 'superscript' : 'subscript') + ' for ' + k);
  }
});

let worker = null;

function setRunning(running) {
  runBtn.disabled = running;
  stopBtn.disabled = !running;
  statusEl.textContent = running ? 'running…' : '';
}

function stopWorker() {
  if (worker) {
    worker.terminate();
    worker = null;
  }
  setRunning(false);
}

runBtn.onclick = () => {
  stopWorker();
  output.textContent = '';
  worker = new Worker('worker.js');
  worker.onmessage = (e) => {
    const m = e.data;
    if (m.type === 'out') {
      output.textContent += m.text;
      output.scrollTop = output.scrollHeight;
    } else if (m.type === 'done') {
      if (m.error) {
        output.textContent += 'error: ' + m.error + '\n';
      }
      stopWorker();
    }
  };
  worker.onerror = (e) => {
    output.textContent += 'worker error: ' + (e.message || e.type) + '\n';
    stopWorker();
  };
  setRunning(true);
  worker.postMessage({ source: editor.value, stdin: stdinEl.value });
};

stopBtn.onclick = () => {
  output.textContent += '[stopped]\n';
  stopWorker();
};

const EXAMPLES = ['cat', 'collatz', 'euclid', 'factorial', 'fibonacci', 'fizzbuzz', 'hello', 'primes', 'quine', 'reverse', 'squares', 'sum', 'truth-machine'];

for (const name of EXAMPLES) {
  const opt = document.createElement('option');
  opt.value = name;
  opt.textContent = name;
  exampleSel.appendChild(opt);
}

exampleSel.onchange = async () => {
  const name = exampleSel.value;
  if (!name) return;
  try {
    const res = await fetch('../examples/' + name + '.formula');
    if (!res.ok) throw new Error(res.status);
    editor.value = await res.text();
  } catch {
    output.textContent += 'could not load example ' + name + '\n';
  }
  exampleSel.value = '';
};
