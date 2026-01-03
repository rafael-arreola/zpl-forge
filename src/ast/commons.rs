/// Represents text justification options in ZPL.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Justification {
    /// Left justification (Default)
    L,
    /// Center justification
    C,
    /// Right justification
    R,
    /// Justified (full width)
    J,
}

impl From<char> for Justification {
    fn from(value: char) -> Self {
        match value {
            'L' => Justification::L,
            'C' => Justification::C,
            'R' => Justification::R,
            'J' => Justification::J,
            _ => {
                tracing::debug!(
                    target: crate::TARGET,
                    "{} is not a valid Justification value, using L as default",
                    value
                );
                Justification::L
            }
        }
    }
}

impl From<Justification> for char {
    fn from(value: Justification) -> Self {
        match value {
            Justification::L => 'L',
            Justification::C => 'C',
            Justification::R => 'R',
            Justification::J => 'J',
        }
    }
}

impl From<Justification> for String {
    fn from(value: Justification) -> Self {
        let c: char = value.into();
        c.to_string()
    }
}

/// Represents a boolean-like state in ZPL (Yes/No).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum YesNo {
    /// Yes ('Y')
    Y,
    /// No ('N')
    N,
}

impl From<char> for YesNo {
    fn from(value: char) -> Self {
        match value {
            'Y' => YesNo::Y,
            'N' => YesNo::N,
            _ => {
                tracing::debug!(
                    target: crate::TARGET,
                    "{} is not a valid YesNo value, using N as default",
                    value
                );
                YesNo::N
            }
        }
    }
}

impl From<YesNo> for char {
    fn from(value: YesNo) -> Self {
        match value {
            YesNo::Y => 'Y',
            YesNo::N => 'N',
        }
    }
}

impl From<bool> for YesNo {
    fn from(value: bool) -> Self {
        if value {
            YesNo::Y
        } else {
            YesNo::N
        }
    }
}

impl From<YesNo> for bool {
    fn from(value: YesNo) -> Self {
        match value {
            YesNo::Y => true,
            YesNo::N => false,
        }
    }
}
