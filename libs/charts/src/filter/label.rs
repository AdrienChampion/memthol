//! Label filters.

use regex::Regex;

use crate::{base::*, filter::FilterExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Kind {
    Contain,
    Exclude,
}
impl fmt::Display for Kind {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Contain => write!(fmt, "contain"),
            Self::Exclude => write!(fmt, "exclude"),
        }
    }
}
impl Kind {
    pub fn all() -> Vec<Kind> {
        vec![Self::Contain, Self::Exclude]
    }
}

/// Label filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LabelFilter {
    /// Retains label lists that contain a label verifying this spec.
    Contain(Vec<LabelSpec>),
    /// Retains label lists that do not contain a label verifying this spec.
    Exclude(Vec<LabelSpec>),
}

/// Constructors.
impl LabelFilter {
    /// `Contain` constructor.
    pub fn contain(specs: Vec<LabelSpec>) -> Self {
        Self::Contain(specs)
    }
    /// `Exclude` constructor.
    pub fn exclude(specs: Vec<LabelSpec>) -> Self {
        Self::Exclude(specs)
    }

    /// Filter's kind.
    pub fn kind(&self) -> (Kind, &Vec<LabelSpec>) {
        match self {
            Self::Contain(specs) => (Kind::Contain, specs),
            Self::Exclude(specs) => (Kind::Exclude, specs),
        }
    }
    /// Filter's kind, mutable version.
    pub fn kind_mut(&mut self) -> (Kind, &mut Vec<LabelSpec>) {
        match self {
            Self::Contain(specs) => (Kind::Contain, specs),
            Self::Exclude(specs) => (Kind::Exclude, specs),
        }
    }

    /// Constructor from a kind.
    pub fn of_kind(kind: Kind, labels: Vec<LabelSpec>) -> Self {
        match kind {
            Kind::Contain => Self::contain(labels),
            Kind::Exclude => Self::exclude(labels),
        }
    }

    /// Label specifications.
    pub fn specs(&self) -> &Vec<LabelSpec> {
        match self {
            Self::Contain(specs) => specs,
            Self::Exclude(specs) => specs,
        }
    }
    /// Label specifications (mutable).
    pub fn specs_mut(&mut self) -> &mut Vec<LabelSpec> {
        match self {
            Self::Contain(specs) => specs,
            Self::Exclude(specs) => specs,
        }
    }

    /// Inserts a specification.
    ///
    /// If the specification is empty, removes the spec at that index.
    pub fn insert(&mut self, index: usize, spec: LabelSpec) {
        let specs = self.specs_mut();
        if spec.is_empty() {
            specs.remove(index);
            ()
        } else {
            specs[index] = spec
        }
    }
}

impl fmt::Display for LabelFilter {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let (kind, specs) = self.kind();
        write!(fmt, "labels {} [", kind)?;
        for spec in specs {
            write!(fmt, " ... {}", spec)?
        }
        write!(fmt, " ... ]")
    }
}

impl FilterExt<[String]> for LabelFilter {
    fn apply(&self, alloc_data: &[String]) -> bool {
        match self {
            Self::Contain(specs) => Self::check_contain(specs, alloc_data),
            Self::Exclude(specs) => !Self::check_contain(specs, alloc_data),
        }
    }
}
impl Default for LabelFilter {
    fn default() -> Self {
        Self::Contain(vec![])
    }
}

impl LabelFilter {
    /// Helper that returns true if some labels verify the input specs.
    fn check_contain(specs: &[LabelSpec], labels: &[String]) -> bool {
        let mut labels = labels.iter();
        let mut specs = specs.iter();

        'next_spec: while let Some(spec) = specs.next() {
            // `can_skip` is true if `spec` does not have to match the next label, it can match
            // labels appearing later in the sequence.
            let (can_skip, spec) = if spec.matches_anything() {
                // We're matching a sequence of anything. Find the next spec that's not an
                // `Anything`.
                let mut spec_opt = None;
                'drain_match_anything: while let Some(spec) = specs.next() {
                    if spec.matches_anything() {
                        continue 'drain_match_anything;
                    } else {
                        spec_opt = Some(spec);
                        break 'drain_match_anything;
                    }
                }

                if let Some(spec) = spec_opt {
                    (true, spec)
                } else {
                    // We're matching anything, and there is no spec to match after that.
                    return true;
                }
            } else {
                // We're matching an actual spec.
                (false, spec)
            };

            'find_match: while let Some(label) = labels.next() {
                if spec.apply(label) {
                    // Found a match.
                    continue 'next_spec;
                } else if can_skip {
                    // `spec` does not have to match right away, keep moving.
                    continue 'find_match;
                } else {
                    return false;
                }
            }

            // Only reachable if there are no more labels.
            return false;
        }

        // Only reachable if there are no more specs and all succeeded. Now we just need to check if
        // there are labels left.
        labels.next().is_none()
    }
}

/// Label specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LabelSpec {
    /// Matches a sequence of arbitrary labels.
    Anything,
    /// An actualy label value.
    Value(String),
    /// A regular expression.
    #[serde(with = "serde_regex")]
    Regex(Regex),
}
impl FilterExt<str> for LabelSpec {
    fn apply(&self, label: &str) -> bool {
        match self {
            LabelSpec::Value(value) => label == value,
            LabelSpec::Regex(regex) => regex.is_match(label),
            LabelSpec::Anything => true,
        }
    }
}

impl fmt::Display for LabelSpec {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Value(label) => label.fmt(fmt),
            Self::Regex(regex) => write!(fmt, "#\"{}\"#", regex),
            Self::Anything => write!(fmt, "..."),
        }
    }
}

impl Default for LabelSpec {
    fn default() -> LabelSpec {
        LabelSpec::Value("my label".into())
    }
}

impl LabelSpec {
    /// Constructor.
    pub fn new<S>(label: S) -> Res<Self>
    where
        S: Into<String>,
    {
        let label = label.into();
        macro_rules! illegal {
            () => {{
                let err: err::Err = format!("illegal regex `{}`", label).into();
                err
            }};
        }
        if label.len() > 2 && &label[0..2] == "#\"" {
            if &label[label.len() - 2..label.len()] != "\"#" {
                bail!(illegal!().chain_err(|| "a regex must end with `\"#`"))
            }

            let regex = Regex::new(&label[2..label.len() - 2])
                .map_err(|e| illegal!().chain_err(|| format!("{}", e)))?;
            Ok(regex.into())
        } else {
            Ok(label.into())
        }
    }

    /// True if the spec is an empty label.
    pub fn is_empty(&self) -> bool {
        match self {
            LabelSpec::Value(s) => s == "",
            LabelSpec::Regex(_) => false,
            LabelSpec::Anything => false,
        }
    }

    /// True if the spec matches anything.
    pub fn matches_anything(&self) -> bool {
        match self {
            Self::Anything => true,
            Self::Value(_) => false,
            Self::Regex(_) => false,
        }
    }
}

impl From<String> for LabelSpec {
    fn from(s: String) -> Self {
        if &s == "..." {
            Self::Anything
        } else {
            Self::Value(s)
        }
    }
}
impl<'a> From<&'a str> for LabelSpec {
    fn from(s: &'a str) -> Self {
        Self::Value(s.into())
    }
}
impl From<Regex> for LabelSpec {
    fn from(re: Regex) -> Self {
        Self::Regex(re)
    }
}

/// An update for a label filter.
pub enum Update {
    /// Change the kind of the filter.
    Kind(Kind),
    /// Add a new label at some position.
    Add(usize),
    /// Replace a label at some position.
    Replace(usize, String),
}
impl fmt::Display for Update {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Kind(kind) => write!(fmt, "kind <- {}", kind),
            Self::Add(index) => write!(fmt, "labels <- add at {}", index),
            Self::Replace(index, spec) => write!(fmt, "labels[{}] <- {}", index, spec),
        }
    }
}

impl LabelFilter {
    /// Updates the filter.
    pub fn update(&mut self, update: Update) -> Res<bool> {
        let (kind, specs) = self.kind_mut();
        let has_changed = match update {
            Update::Kind(nu_kind) => {
                if nu_kind != kind {
                    Self::of_kind(nu_kind, specs.clone());
                    true
                } else {
                    false
                }
            }
            Update::Add(index) => {
                specs.insert(index, LabelSpec::default());
                true
            }
            Update::Replace(index, spec) => {
                let spec =
                    LabelSpec::new(spec).chain_err(|| "while replacing label spec in filter")?;
                specs[index] = spec;
                true
            }
        };
        Ok(has_changed)
    }
}
