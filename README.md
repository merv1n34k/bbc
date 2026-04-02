# BBC -- Better BC

A batteries-included CLI calculator with arbitrary-precision arithmetic, physical units, dimensional analysis, base conversion, and more. Single Rust binary.

## Install

```bash
cargo install --path crates/bbc-cli
```

Or build from source:

```bash
make build
# binary at target/release/bbc
```

## Usage

```bash
# Interactive REPL
bbc

# Single expression
bbc -e '2 + 3'

# From file
bbc -f script.bbc

# From stdin
echo '16xFF + 1' | bbc
```

## Features

### Arbitrary-precision arithmetic

All numbers are stored as exact rationals (no floating-point rounding):

```
> 1/3 * 3
1
> 1/7 + 1/7 + 1/7 + 1/7 + 1/7 + 1/7 + 1/7
1
```

### Physical units and dimensional analysis

Seven-dimensional SI system (length, mass, time, current, temperature, amount, luminosity):

```
> 5 [kg] * 9.8 [m*s^-2]
49 [N]
> 100 [km] -> [mi]
62.1371192... [mi]
> 3 [km] + 500 [m]
3500 [m]
> 9.8 [m*s^-2] + 1 [K]
error: dimension mismatch
```

### Base conversion

Input and output in any base (2-36):

```
> 16xFF
255
> 2x1010
10
> 255 -> 16x
16xFF
> 16xFF >> 2 -> 16x
16x3F
> 16xFF.8
255.5
```

### Physical constants

Built-in constants with proper units:

```
> c
299792458 [m*s^-1]
> h
6.62607015e-34 [m^2*kg*s^-1]
> pi
3.14159265358979...
```

Constants are immutable:

```
> c = 5
error: cannot reassign constant 'c'
```

### Variables

```
> x = 42
42
> x * 2
84
> const g = 9.8 [m*s^-2]
9.8 [m*s^-2]
> g = 10
error: cannot reassign constant 'g'
```

### LaTeX input

Paste LaTeX expressions directly:

```
> \frac{1}{3} + \frac{1}{6}
0.5
> \sqrt{144}
12
> \sqrt[3]{27}
3
> 2 \cdot \pi
6.28318530717958...
```

### Built-in functions

```
sin, cos, tan, asin, acos, atan
sqrt, cbrt, exp, ln, log
abs, floor, ceil, round
min, max
```

### Bitwise operations

```
> 16xFF & 16x0F
15
> 16xFF | 16x100
511
> 1 << 8
256
> ~0
-1
```

### Boolean operations

```
> true && false
false
> 3 < 5
true
> 3 == 3
true
```

## CLI Options

```
--expr, -e <EXPR>     Evaluate a single expression
--file, -f <FILE>     Evaluate a file
--obase <N>           Output base (2-36, default 10)
--scale <N>           Decimal precision (default 20)
--units <SETS>        Load unit sets (comma-separated: imperial,scientific,engineering,kitchen,biology)
--sigfig              Enable significant figures tracking
--strict              Strict SI mode (bare SI output, no conversion)
```

### Unit sets

Additional units can be loaded with `--units`:

- **imperial** -- ft, in, yd, mi, lb, oz, gal, psi, hp, acre, ...
- **scientific** -- eV, Angstrom, au, ly, pc, Da, torr, Gauss, ...
- **engineering** -- psi, cP, cfm, gpm, Ah, Wh, bit, byte, KB, MB, GB, ...
- **kitchen** -- cup, tbs, tsp, floz, qt, pt
- **biology** -- bp, kbp, Da, kDa, MDa

### Sigfig mode

Tracks significant figures through calculations:

```bash
bbc --sigfig -e '3.14 * 2.0'
# 6.3  (2 sigfigs from 2.0)
```

### Strict SI mode

Forces output in bare SI base units only:

```bash
bbc --strict -e '5 [mi]'
# 8046.72 [m]
bbc --strict -e '100 [km] -> [mi]'
# error: strict mode: explicit conversion not allowed
```

## License

Distributed under MIT licence, see `LICENSE` for more.
