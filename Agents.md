# Compiler Development Rules

## 1. Feature Development Flow

- Every new feature MUST include:
  - Unit tests (semantic + parser if applies)
  - Minimal working example in `/examples`
  - Type system validation if applicable

- No feature is considered complete without tests passing.

---

## 2. Branching Policy

- No direct commits to `master`.
- Every feature must be developed in a separate branch:
  - feature/<name>
  - fix/<name>
  - refactor/<name>

- Branch naming must reflect intent clearly.

---

## 3. Architecture Rules

- The compiler is strictly modular:
  - parser/
  - semantic/
  - codegen/
  - bin/

- No cross-layer dependencies backwards.
  (e.g. semantic cannot depend on codegen)

---

## 4. Type System Invariants

- All types MUST be resolved through TypeTable.
- SemanticType is only a reference to TypeId.
- No raw string type comparisons allowed in semantic phase.
- Function signatures are immutable once registered.

---

## 5. Function & Method Rules

- Functions are stored in FunctionSymbol table.
- Methods are functions with optional receiver.
- Method resolution must follow:
  1. current type
  2. parent types (if any)
  3. error if not found

---

## 6. Testing Requirements

- Every feature must include:
  - positive test cases
  - negative test cases
  - edge cases if applicable

- Regression tests must be added if bug is fixed.

---

## 7. Code Quality Rules

- No duplicated logic across semantic/codegen layers.
- Prefer data-driven design (TypeTable, SymbolTable).
- Avoid hardcoded type checks in logic.

---

## 8. Build & Binary Layer

- src/bin must only orchestrate compiler execution.
- It must NOT contain business logic.
- CLI/UI depends on compiler, never the reverse.

---

## 9. Documentation

- README updates are required only if:
  - syntax changes
  - new language feature exposed to users

- Internal refactors do NOT require README changes.