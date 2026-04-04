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
> pi
3.14159265358979...
> k_B
0 [m^2*kg*s^-2*K^-1]
```

Small constants like `h` and `k_B` need `--view scientific` to display properly:

```bash
bbc --view scientific -e 'h'
# 6.62607015e-34 [m^2*kg*s^-1]
bbc --view scientific -e 'k_B'
# 1.380649e-23 [m^2*kg*s^-2*K^-1]
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
sin, cos, tan, asin, acos, atan, atan2
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
--view <VIEWS>        Display views (comma-separated: scientific,adjust,strict)
--sigfig              Enable significant figures tracking
```

### Unit sets

Additional units can be loaded with `--units`:

- **imperial** -- ft, in, yd, mi, lb, oz, gal, psi, hp, acre, ...
- **scientific** -- eV, Angstrom, au, ly, pc, Da, torr, Gauss, ...
- **engineering** -- psi, cP, cfm, gpm, Ah, Wh, bit, byte, KB, MB, GB, ...
- **kitchen** -- cup, tbs, tsp, floz, qt, pt
- **biology** -- bp, kbp, Da, kDa, MDa

### View modes

Views control how results are displayed without affecting calculations. `adjust` is on by default.

- **adjust** -- auto-prefix (km, mA, uF) and derived unit names (N, J, W)
- **scientific** -- scientific notation for very large/small numbers
- **strict** -- raw SI base units only (no prefix, no derived names)

`adjust` and `strict` are mutually exclusive (enabling one disables the other).

```bash
# Auto-prefix (adjust is on by default)
bbc -e '1500 [m]'
# 1.5 [km]
bbc -e '0.000025 [m]'
# 25 [um]

# Scientific notation for extreme values
bbc --view scientific -e 'N_A'
# 6.02214076e23 [mol^-1]

# Strict: raw SI base units
bbc --view strict -e '5 [km]'
# 5000 [m]
```

At the REPL, use the `view` command to list, enable, or disable views:

```
> view
active: adjust
available: scientific strict
> view scientific
> view -adjust
```

### Runtime commands

Manage units interactively during a REPL session:

```
> units                           # list loaded/available unit sets
> units imperial                  # load a unit set
> units +scientific               # load (same as above)
> units -imperial                 # unload a unit set
> unit ft                         # show info about a unit
> unit smoot = 67 [in]
> 364.4 [smoot] -> [m]
620.135... [m]
```

### Sigfig mode

Tracks significant figures through calculations:

```bash
bbc --sigfig -e '3.14 * 2.0'
# 6.3  (2 sigfigs from 2.0)
```

## License

Distributed under MIT licence, see `LICENSE` for more.
