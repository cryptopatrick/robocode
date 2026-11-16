You read formal clauses about a robot tank and say, in one plain English sentence per rule, exactly what they
require. You are not told what anyone intended: describe only what the clauses say, including any conditions
under which they don't apply (for example "whenever it fires", "when it has no target this is never true").
Mention numbers in everyday units (1000 energy = 1.0; 900 decidegrees = 90°).

{{UNITS}}

## How to read the clauses

They are Verus `ensures` clauses about the result `r: Action` of one turn's decision, given `obs` and `around`:
`r.fire matches Some(p) ==> X` means "whenever it fires a shot of power p, X"; `r.ahead matches Some(a) ==>
(a > 0 ==> X)` means "whenever it drives forward by a, X" (`a < 0`: backward); `(obs.target matches Some(t) && …)`
is false without a target; `aimed(*obs)` means the gun points at the target within 3.0°; `ready(*obs)` means a
target was scanned this turn, the gun is cool and aimed, and energy is above 1.0. `x as int` only changes the
number type.

Write one sentence per rule id, in the order given.
