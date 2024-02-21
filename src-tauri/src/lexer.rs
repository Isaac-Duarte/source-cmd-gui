#[derive(Debug, PartialEq)]
pub enum Token {
    Number(f64),
    Operator(char),
    Function(String),
    LeftParenthesis,
    RightParenthesis,
    Constant(String),
}
impl Token {
    pub fn is_number(&self) -> bool {
        matches!(self, Token::Number(_))
    }

    pub fn is_parathesis(&self) -> bool {
        matches!(self, Token::LeftParenthesis | Token::RightParenthesis)
    }
}

const FUNCTIONS: [&str; 22] = [
    "sqrt", "abs", "exp", "ln", "log10", "sin", "cos", "tan", "asin", "acos", "atan", "atan2",
    "sinh", "cosh", "tanh", "asinh", "acosh", "atanh", "floor", "ceil", "round", "signum",
];

pub fn tokenize(expr: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = expr.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '0'..='9' => {
                let mut num = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_ascii_digit() || next_ch == '.' {
                        num.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Number(num.parse().unwrap()));
            }
            '+' | '-' | '*' | '/' | '^' | '%' | '!' => {
                tokens.push(Token::Operator(chars.next().unwrap()));
            }
            'x' => {
                tokens.push(Token::Operator('*'));
                chars.next();
            }
            '(' => {
                tokens.push(Token::LeftParenthesis);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RightParenthesis);
                chars.next();
            }
            'a'..='z' => {
                let mut name = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphabetic() {
                        name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if ["pi", "e"].contains(&name.as_str()) {
                    tokens.push(Token::Constant(name));
                } else if FUNCTIONS.contains(&name.as_str()) {
                    tokens.push(Token::Function(name));
                }
            }
            ' ' => {
                chars.next();
            }
            _ => {
                if !tokens.is_empty() {
                    break;
                }
                chars.next();
            } // Skip other characters like spaces
        }
    }

    tokens
}

pub fn to_string(tokens: &Vec<Token>) -> String {
    let mut string = String::new();

    for token in tokens {
        match token {
            Token::Number(num) => string.push_str(&num.to_string()),
            Token::Operator(op) => string.push(*op),
            Token::Function(name) => string.push_str(name),
            Token::LeftParenthesis => string.push('('),
            Token::RightParenthesis => string.push(')'),
            Token::Constant(name) => string.push_str(name),
        }
    }

    string
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let expr = "This is a test expression: 3 + 4 * 2 / ( 1 - 5 ) ^ 2 ^ 3";
        let expected_tokens = vec![
            Token::Number(3.0),
            Token::Operator('+'),
            Token::Number(4.0),
            Token::Operator('*'),
            Token::Number(2.0),
            Token::Operator('/'),
            Token::LeftParenthesis,
            Token::Number(1.0),
            Token::Operator('-'),
            Token::Number(5.0),
            Token::RightParenthesis,
            Token::Operator('^'),
            Token::Number(2.0),
            Token::Operator('^'),
            Token::Number(3.0),
        ];
        assert_eq!(tokenize(expr), expected_tokens);
        println!("{:?}", to_string(&tokenize(expr)));
    }
}
