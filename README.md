# qed

qed is an esoteric programming language that has zero english keywords and instead uses pure math symbols. there is no `main`, nothing. a program is a derivation, and the final "therefore" line is what gets printed.
you can only use qed if you are a math major. if you are not, please do not taint qed's name by trying to write something on it.

## core

a qed program is a sequence of definitions followed by a final output line.

definitions use mathematical definition notation:

```qed
x := e
```

```qed
f(x) := e
```

the program's result is introduced by the "therefore" symbol:

```qed
f(x) := x² + 1
∴ f(3) ∎
```

the program above prints:

```text
10
```

### names

names are single letters. longer runs are juxtaposition, not names: `fb` means `f` followed by `b`. that is the whole trick behind fizzbuzz below.

reserved words (`chi`, `forall`, and so on, see the alias table) are the exception. they are ascii spellings of symbols, not identifiers, so they do not count against the one-letter rule.

## built-ins

to make io possible while keeping the source looking like math, qed has a few special mathematical functions and objects.

| notation | meaning |
|---|---|
| `x := e` | define `x` |
| `f(x) := e` | define function `f` |
| `∴ e ∎` | output `e` (truth values print as `⊤`/`⊥`) |
| `χ(n)` | character with unicode code point `n` (`χ` of a character is itself) |
| `δ(n)` | decimal representation of natural number `n` |
| `ε` | empty string |
| `s ⊕ t` | string concatenation |
| `st` | also string concatenation, when context is textual |
| `⨁ᵢ₌ₐᵇ sᵢ` | concatenation of a sequence of strings |
| `𝒞` | entire standard input as a character sequence |
| `𝒩` | standard input parsed as a sequence of natural numbers |
| `μx. P(x)` | least natural `x` satisfying `P(x)`; diverges if none exists |
| `𝒮` | the source of the program. i added this so i can cheat quine |

strings are elements of a free monoid. juxtaposition means concatenation, and exponentiation means repetition:

```qed
χ(108)² = χ(108)χ(108)
```

## unicode vs ascii

the canonical syntax is actual unicode math. that is the real language.

if you are a wuss, you can use ascii characters instead, and they will all map to the same lexical representation. the parser normalizes both forms into the same internal tokens, so these three programs are identical:

```qed
∴ ⨁ᵢ₌₁¹⁴ χ(wᵢ) ∎
```

```qed
∴ ⨁_{i=1}^{14} χ(w_i) ∎
```

```qed
|- bigoplus_{i=1..14} chi(w_i) .
```

some useful aliases:

| unicode | ascii |
|---|---|
| `∴` | `\|-` |
| `∎` | `.` |
| `χ` | `chi` |
| `δ` | `dec` |
| `ε` | `eps` |
| `⊕` | `++` |
| `⨁` | `bigoplus` |
| `𝒞` | `C` |
| `𝒩` | `N` |
| `𝒮` | `source` |
| `μx.` | `mu x.` |
| `∧` | `&` |
| `∨` | `or` |
| `¬` | `~` |
| `⊤` | `true` |
| `⊥` | `false` |
| `∀` | `forall` |
| `∃` | `exists` |
| `∈` | `in` |
| `∣` | `\|` or `divides` (single bar; `\|-` stays therefore, so write `|s| - i` with spaces in ascii) |
| `∤` | `notdiv` |
| `≤` | `<=` |
| `≥` | `>=` |
| `<` | `<` |
| `>` | `>` |
| `:` | `:` (separates the `∀`-set from its predicate; `:=` stays define) |
| `…` | `...` (`..` stays range) |
| `∞` | `inf` |
| `n!` | `n!` (postfix factorial) |
| `\|s\|` | `\|s\|` length of sequence `s` |
| `⌊√n⌋` | `floor(sqrt(n))` |
| `⌈√n⌉` | `ceil(sqrt(n))` |
| `≠` | `!=` |
| `≡` | `equiv(a, b, n)` for `a ≡ b mod n` |
| `∏` | `prod` |
| `∪` | `union` |
| `∩` | `inter` |
| `\` | `\` (set difference) |
| `⊆` | `subset` |
| `∘` | `compose` (`(f∘g)(x)` is `f(g(x))`, `T^{∘k}` iterates `T` `k` times) |
| `ℳ{…}` | `meta{…}` |
| `⊨` | `\|=` |
| `⊢` | `\|-` (in metalanguage position only) |
| `↓` | `halts` |

unicode subscripting is optional sugar. `wᵢ` and `w_i` mean the same thing. `∑ᵢ₌₁¹⁰⁰` and `∑_{i=1}^{100}` mean the same thing. use whichever you can actually type without crying (you will cry regardless).

people who get caught using the ascii variant will get a 300 reais fine. this cannot be debated.

## conditions

there is no `if`. branches use guards separated by `|`:

```qed
f(n) := { 0 | n = 0
          1 | n ≠ 0 }
∴ f(42) ∎
```

prints:

```text
1
```

the first branch whose guard holds wins. a branch with no guard always wins. no true guard is a runtime error, reported as `∄ true guard`.

## metalanguage

`ℳ` switches into the metalanguage: claims about the program instead of parts of it. `ℳ{ ... }` is unparsed prose, which is how you write comments:

```qed
ℳ{ f counts up from here, trust me }
f(x) := x + 1
∴ f(1) ∎
```

a metalanguage statement can also carry a judgment:

| written | meaning |
|---|---|
| `ℳ ⊢ e` | remark claiming `e` |
| `ℳ ↓ f` | remark claiming `f` terminates |
| `ℳ ⊨ e` | checked claim: `e` must evaluate to true |
| `ℳ ⊥` | the derivation reaches bottom |
| `ℳ ∎` | the metalanguage's own proof is complete |

only `⊨` does anything: the program fails unless its predicate holds. everything else is held and ignored. assertions run after all definitions, before the goal, so this passes no matter where the statement sits:

```qed
ℳ ⊨ F(5) = 6
F(x) := x + 1
∴ F(1) ∎
```

anything else after `ℳ` is held as an uninterpreted remark. levels stack: `ℳ₀` talks about the program, `ℳ₁` talks about `ℳ₀`'s claims. they change nothing yet, they just keep the tower straight.

## hello, cruel world...

in qed, there are no ordinary string literals, because string literals aren't very mathematical and are useless anyways. text must be constructed from numbers using `χ`.

a "Hello, cruel world...", with a trailing newline:

```qed
w := χ(72,101,108,108,111,44,32,99,114,117,101,108,32,119,111,114,108,100,46,46,46,10)
∴ ⨁ᵢ₌₁²² χ(wᵢ) ∎
```

as we discussed earlier, `χ` turns unicode code into characters, so `72` is `H`, `101` is `e`, and so on. `w` is therefore the sequence of characters making up the message. `⨁` means "concatenate all of these characters" and `∴` marks the expression qed should output.

## example programs

### truth machine

if input is `0`, output `0`. if input is `1`, output `1` infinitely.

```qed
∴ { χ(48) | N₁ = 0
    ⨁ₙ₌₁∞ χ(49) | N₁ = 1 } ∎
```

### quine

```qed
∴ 𝒮 ∎
```

outputs its own source, byte for byte.

### cat

print the entire input unchanged.

```qed
∴ 𝒞 ∎
```

### reverse input

assuming finite input, concatenate the input characters in reverse order. `|s|` is the length, or cardinality, of sequence `s`. sequences are one-indexed: `s₁` is the first element.

```qed
∴ ⨁ᵢ₌₁^{∣𝒞∣} 𝒞_{∣𝒞∣-i+1} ∎
```

ascii version (note the spaces around `-`, otherwise `|-` lexes as therefore):

```qed
|- bigoplus_{i=1}^{|C|} C_{|C| - i+1} .
```

### sum all input numbers

if standard input contains whitespace-separated natural numbers:

```qed
∴ ∑_{x∈𝒩} x ∎
```

for input:

```text
1 2 3 4
```

it prints:

```text
10
```

### factorial

`n!` is postfix factorial:

```qed
F(n) := { 1 | n = 0
          nF(n-1) | n > 0 }
∴ F(5) ∎
```

prints:

```text
120
```

so `∴ 5! ∎` also prints `120`.

### fibonacci sequence

using recurrence notation, print the first eleven fibonacci numbers:

```qed
F₀ := 0
F₁ := 1
F_{n+2} := F_{n+1} + F_n
∴ ⨁ᵢ₌₀¹⁰ (δ(Fᵢ) ⊕ χ(10)) ∎
```

prints:

```text
0
1
1
2
3
5
8
13
21
34
55
```

### print squares

```qed
∴ ⨁ₙ₌₁¹⁰ (δ(n²) ⊕ χ(10)) ∎
```

prints:

```text
1
4
9
16
25
36
49
64
81
100
```

### fizzbuzz

define the strings mathematically from code points, so `f` is "Fizz" and `b` is "Buzz":

```qed
f := χ(70)χ(105)χ(122)χ(122)
b := χ(66)χ(117)χ(122)χ(122)
g(n) := { fb | 15 ∣ n
          f | 3 ∣ n
          b | 5 ∣ n
          δ(n) | ⊤ }
∴ ⨁ₙ₌₁¹⁰⁰ (g(n) ⊕ χ(10)) ∎
```

this prints the usual fizzbuzz sequence.

### prime numbers up to 100

define a primality predicate:

```qed
P(n) := { 1 | n ≥ 2 ∧ ∀d ∈ {2,…,⌊√n⌋} : d ∤ n
          0 | ⊤ }
∴ ⨁ₙ₌₁¹⁰⁰ { δ(n) ⊕ χ(10) | P(n) = 1
            ε | ⊤ } ∎
```

ascii version:

```qed
P(n) := { 1 | n >= 2 & forall d in {2,...,floor(sqrt(n))} : d notdiv n
          0 | true }
|- bigoplus_{n=1..100} { dec(n) ++ chi(10) | P(n) = 1
            eps | true } .
```

prints:

```text
2
3
5
7
11
13
17
19
23
29
31
37
41
43
47
53
59
61
67
71
73
79
83
89
97
```

### euclidean algorithm

a very natural qed program.

```qed
G(a,0) := a
G(a,b) := G(b, a - ⌊a/b⌋b)
∴ G(48,18) ∎
```

prints:

```text
6
```

### collatz sequence

define the collatz step:

```qed
T(n) := { n/2 | 2 ∣ n
          3n+1 | 2 ∤ n }
```

let `T^{∘k}` mean `T` composed with itself `k` times. `^` alone is exponentiation; the `∘` is what makes it iteration. define the stopping time using unbounded search:

```qed
τ(n) := μk. T^{∘k}(n) = 1
∴ ⨁ᵢ₌₀^{τ(27)} (δ(T^{∘i}(27)) ⊕ χ(10)) ∎
```

this prints all 112 terms from 27 down to 1.

### unbounded search

`μx. P(x)` searches for the least natural number satisfying `P(x)`:

```qed
∴ μx. x² > 100 ∎
```

evaluates to:

```text
11
```

if no such `x` exists, the program diverges. this gives qed turing-complete power. for example, this program hangs forever:

```qed
∴ μx. x ≠ x ∎
```

it is the computational equivalent of an impossible proof obligation.

## legality

these snippets are very proprietary and you might get sued if you use any of them commercially, or even at all. i have good lawyers, don't even try.

## misc thoughts and whatever

my first "real" rust program. the code is not perfect, and i have cried many times when making this. i learned a lot, though. and i made it in like a week
it's licensed under WTFPL, because i don't care what you do with this.
i hope you write many programs in this beautiful language.
