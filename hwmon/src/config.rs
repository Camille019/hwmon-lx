// SPDX-FileCopyrightText: 2018 Camille019
// SPDX-License-Identifier: MPL-2.0

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Operator {
    Add,
    Sub,
    Multiply,
    Divide,
}

impl Operator {
    pub(crate) fn eval(&self, left: f64, right: f64) -> f64 {
        match self {
            Operator::Add => left + right,
            Operator::Sub => left - right,
            Operator::Multiply => left * right,
            Operator::Divide => left / right,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Function {
    Inv,
    Exp,
    Ln,
}

impl Function {
    pub(crate) fn eval(&self, arg: f64) -> f64 {
        match self {
            Function::Inv => -arg,
            Function::Exp => arg.exp(),
            Function::Ln => arg.ln(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum Expr {
    Fn(Function, Box<Expr>),
    Op(Operator, Box<Expr>, Box<Expr>),
    Literal(f64),
    #[default]
    Raw,
}

impl Expr {
    pub(crate) fn eval(&self, raw: f64) -> f64 {
        match self {
            Expr::Fn(inner, expr) => inner.eval(expr.eval(raw)),
            Expr::Op(inner, left, right) => inner.eval(left.eval(raw), right.eval(raw)),
            Expr::Literal(inner) => *inner,
            Expr::Raw => raw,
        }
    }
}

//struct ChipName {
//    prefix: String,
//    bus: Bus,
//    address: u32,
//}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CfgFile {
    buses: Vec<StmtBus>,
    chips: Vec<StmtChip>,
}

impl CfgFile {
    pub(crate) fn new(buses: Vec<StmtBus>, chips: Vec<StmtChip>) -> Self {
        Self { buses, chips }
    }
    /// Find the configuration statement matching the given chip name.
    ///
    /// Chip names in the configuration file may contain `*` wildcards, as
    /// described in sensors.conf(5). When several statements match, the
    /// last one in the file is returned, as libsensors does.
    pub(crate) fn chip_config(&self, chip_name: &str) -> Option<&StmtChip> {
        self.chips
            .iter()
            .rev()
            .find(|chip| chip.matches_by_name(chip_name))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StmtBus {
    number: String,
    adapter: String,
}

impl StmtBus {
    pub(crate) fn new(number: String, adapter: String) -> Self {
        Self { number, adapter }
    }
    pub(crate) fn number(&self) -> &str {
        self.number.as_ref()
    }
    pub(crate) fn adapter(&self) -> &str {
        self.adapter.as_ref()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StmtChip {
    pub(crate) names: Vec<String>,
    pub(crate) labels: Vec<StmtLabel>,
    pub(crate) sets: Vec<StmtSet>,
    pub(crate) computes: Vec<StmtCompute>,
    pub(crate) ignores: Vec<StmtIgnore>,
}

impl StmtChip {
    pub(crate) fn sets(&self) -> &Vec<StmtSet> {
        self.sets.as_ref()
    }

    /// Return the label configured for the given feature name, if any.
    ///
    /// When several `label` statements apply to the same name, the first one
    /// in the file is returned.
    pub(crate) fn label_of(&self, feature_name: &str) -> Option<&str> {
        self.labels
            .iter()
            .find(|label| label.name() == feature_name)
            .map(StmtLabel::value)
    }

    pub(crate) fn is_ignored(&self, feature_name: &str) -> bool {
        self.ignores.iter().any(|stmt| stmt.name() == feature_name)
    }

    /// Return the compute statement configured for the given feature name, if any.
    ///
    /// When several `compute` statements apply to the same name, the first one
    /// in the file is returned.
    pub(crate) fn compute_of(&self, feature_name: &str) -> Option<&StmtCompute> {
        self.computes
            .iter()
            .find(|compute| compute.name() == feature_name)
    }

    fn matches_by_name(&self, name: &str) -> bool {
        self.names
            .iter()
            .any(|pattern| chip_name_matches(pattern, name))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StmtLabel {
    name: String,
    value: String,
}

impl StmtLabel {
    pub(crate) fn new(name: String, value: String) -> Self {
        Self { name, value }
    }
    pub(crate) fn name(&self) -> &str {
        self.name.as_ref()
    }
    pub(crate) fn value(&self) -> &str {
        self.value.as_ref()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StmtIgnore {
    name: String,
}

impl StmtIgnore {
    pub(crate) fn new(name: String) -> Self {
        Self { name }
    }
    pub(crate) fn name(&self) -> &str {
        self.name.as_ref()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StmtCompute {
    name: String,
    statement: String,
    from_proc: Expr,
    to_proc: Expr,
}

impl StmtCompute {
    pub(crate) fn new(name: String, statement: String, from_proc: Expr, to_proc: Expr) -> Self {
        Self {
            name,
            statement,
            from_proc,
            to_proc,
        }
    }
    pub(crate) fn name(&self) -> &str {
        self.name.as_ref()
    }
    /// The statement as written in the configuration file.
    pub(crate) fn statement(&self) -> &str {
        self.statement.as_ref()
    }
    /// Evaluate the expression converting a raw value to a real-world value.
    pub(crate) fn eval_from(&self, raw: f64) -> f64 {
        self.from_proc.eval(raw)
    }
    /// Evaluate the expression converting a real-world value back to a raw value.
    pub(crate) fn eval_to(&self, value: f64) -> f64 {
        self.to_proc.eval(value)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct StmtSet {
    name: String,
    value: Expr,
}

impl StmtSet {
    pub(crate) fn new(name: String, value: Expr) -> Self {
        Self { name, value }
    }
    pub(crate) fn name(&self) -> &str {
        self.name.as_ref()
    }
    pub(crate) fn value(&self) -> &Expr {
        &self.value
    }
}

/// Match a chip name pattern against a chip name.
///
/// `*` matches any sequence of characters, including none, as in
/// sensors.conf(5). All other characters are matched literally.
fn chip_name_matches(pattern: &str, chip_name: &str) -> bool {
    let pattern: Vec<char> = pattern.chars().collect();
    let chip_name: Vec<char> = chip_name.chars().collect();

    let mut pattern_index = 0;
    let mut name_index = 0;
    let mut star: Option<usize> = None;
    let mut star_name_index = 0;

    while name_index < chip_name.len() {
        if pattern_index < pattern.len() && pattern[pattern_index] == chip_name[name_index] {
            pattern_index += 1;
            name_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == '*' {
            star = Some(pattern_index);
            star_name_index = name_index;
            pattern_index += 1;
        } else if let Some(star_index) = star {
            pattern_index = star_index + 1;
            star_name_index += 1;
            name_index = star_name_index;
        } else {
            return false;
        }
    }

    pattern[pattern_index..].iter().all(|c| *c == '*')
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chip_name_matches_wildcards() {
        assert!(chip_name_matches("*", "lm78-isa-0290"));
        assert!(chip_name_matches("lm78-*", "lm78-isa-0290"));
        assert!(chip_name_matches("*-isa-*", "lm78-isa-0290"));
        assert!(chip_name_matches("*-0290", "lm78-isa-0290"));
        assert!(chip_name_matches("lm78-*-0290", "lm78-isa-0290"));
        assert!(chip_name_matches("lm78-isa-0290", "lm78-isa-0290"));

        assert!(!chip_name_matches("lm79-*", "lm78-isa-0290"));
        assert!(!chip_name_matches("*isa", "lm78-isa-0290"));
        assert!(!chip_name_matches("lm78-isa", "lm78-isa-0290"));
        assert!(!chip_name_matches("", "lm78-isa-0290"));
    }
}
