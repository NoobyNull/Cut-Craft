//! Calculation edit boxes: a number field may hold a sum, worked out when it is entered
//! (Aspire's help: Calculation Edit Boxes). `+ - * / ^`, brackets, and letters for stored
//! values: W or X the material width, H or Y its height, T or Z its thickness, P π, I to
//! inches (÷ 25.4), M to millimetres (× 25.4), and `'` after a number for feet (× 12).

/// What the letters stand for, in the units the field shows.
#[derive(Clone, Copy, Debug, Default)]
pub struct Values {
    pub width: f64,
    pub height: f64,
    pub thickness: f64,
}

/// The value of `text`, a number or a sum; a trailing `=` is allowed. `None` when it is not one.
pub fn evaluate(text: &str, values: &Values) -> Option<f64> {
    let text = text.trim().trim_end_matches('=');
    let tokens: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut p = Parser {
        tokens: &tokens,
        at: 0,
        values,
    };
    let v = p.sum()?;
    (p.at == tokens.len() && v.is_finite()).then_some(v)
}

struct Parser<'a> {
    tokens: &'a [char],
    at: usize,
    values: &'a Values,
}

impl Parser<'_> {
    fn peek(&self) -> Option<char> {
        self.tokens.get(self.at).copied()
    }

    fn eat(&mut self, c: char) -> bool {
        let found = self.peek() == Some(c);
        if found {
            self.at += 1;
        }
        found
    }

    /// Terms joined by `+` and `-`.
    fn sum(&mut self) -> Option<f64> {
        let mut v = self.product()?;
        loop {
            if self.eat('+') {
                v += self.product()?;
            } else if self.eat('-') {
                v -= self.product()?;
            } else {
                return Some(v);
            }
        }
    }

    /// Signed powers joined by `*` and `/`.
    fn product(&mut self) -> Option<f64> {
        let mut v = self.unary()?;
        loop {
            if self.eat('*') {
                v *= self.unary()?;
            } else if self.eat('/') {
                v /= self.unary()?;
            } else {
                return Some(v);
            }
        }
    }

    /// A leading sign applies after the power: -2^2 is -4.
    fn unary(&mut self) -> Option<f64> {
        if self.eat('-') {
            return Some(-self.unary()?);
        }
        if self.eat('+') {
            return self.unary();
        }
        self.power()
    }

    /// `a ^ b`, right to left.
    fn power(&mut self) -> Option<f64> {
        let base = self.feet()?;
        if self.eat('^') {
            return Some(base.powf(self.unary()?));
        }
        Some(base)
    }

    /// A number, or with `'` after it that many feet in inches: 2' is 24.
    fn feet(&mut self) -> Option<f64> {
        let v = self.atom()?;
        Some(if self.eat('\'') { v * 12.0 } else { v })
    }

    fn atom(&mut self) -> Option<f64> {
        if self.eat('(') {
            let v = self.sum()?;
            return self.eat(')').then_some(v);
        }
        let c = self.peek()?;
        if c.is_ascii_digit() || c == '.' {
            let start = self.at;
            while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '.') {
                self.at += 1;
            }
            let number: String = self.tokens[start..self.at].iter().collect();
            return number.parse().ok();
        }
        self.at += 1;
        let v = self.values;
        match c.to_ascii_lowercase() {
            'w' | 'x' => Some(v.width),
            'h' | 'y' => Some(v.height),
            't' | 'z' => Some(v.thickness),
            'p' => Some(std::f64::consts::PI),
            'i' => Some(1.0 / 25.4),
            'm' => Some(25.4),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOB: Values = Values {
        width: 600.0,
        height: 300.0,
        thickness: 18.0,
    };

    fn calc(text: &str) -> Option<f64> {
        evaluate(text, &JOB)
    }

    #[test]
    fn sums_follow_the_usual_order() {
        assert_eq!(calc("12.5"), Some(12.5));
        assert_eq!(calc("3+(4/5)"), Some(3.8));
        assert_eq!(calc("2 + 3 * 4 ="), Some(14.0));
        assert_eq!(calc("-2^2"), Some(-4.0));
        assert_eq!(calc("2^3^2"), Some(512.0));
        assert_eq!(calc("(1+2)*-3"), Some(-9.0));
    }

    #[test]
    fn letters_stand_for_the_material_and_constants() {
        assert_eq!(calc("w/2="), Some(300.0));
        assert_eq!(calc("H*2"), Some(600.0));
        assert_eq!(calc("t-0.25"), Some(17.75));
        assert_eq!(calc("x+y+z"), Some(918.0));
        assert!((calc("P*10^2").unwrap() - 314.159_265).abs() < 1e-5);
        assert!((calc("25.4*i").unwrap() - 1.0).abs() < 1e-12);
        assert!((calc("2*M").unwrap() - 50.8).abs() < 1e-12);
        assert_eq!(calc("2'+10"), Some(34.0), "feet and inches");
    }

    #[test]
    fn anything_else_is_not_a_number() {
        for bad in ["", "2+", "(1", "1)", "q", "1..2", "1/0", "abc"] {
            assert_eq!(calc(bad), None, "{bad:?}");
        }
    }
}
