#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Op {
    Add,
    Neg,
    Mul,
    Div,
    Exp,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ASTNode {
    Number(f64),
    Negate(Box<ASTNode>),
    Binary(Box<ASTNode>, Op, Box<ASTNode>),
}

impl ASTNode {
    pub fn eval(&self) -> f64 {
        match self {
            Self::Number(num) => *num,
            Self::Negate(node) => -node.eval(),
            Self::Binary(node_1, op, node_2) => match op {
                Op::Add => node_1.eval() + node_2.eval(),
                Op::Neg => node_1.eval() - node_2.eval(),
                Op::Mul => node_1.eval() * node_2.eval(),
                Op::Div => node_1.eval() / node_2.eval(),
                Op::Exp => node_1.eval().powf(node_2.eval()),
            },
        }
    }
}
