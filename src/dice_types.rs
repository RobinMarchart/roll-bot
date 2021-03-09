#[derive(Debug,PartialEq, Eq,Clone, Copy)]
pub enum DiceType{
    Number(u32),
    Fudge,
    Multiply(u32)
}

#[derive(Debug,PartialEq, Eq, Clone, Copy)]
pub struct Dice{
    pub throws: u32,
    pub dice: DiceType
}

#[derive(Debug,PartialEq, Eq,Clone, Copy)]
pub enum Filter{
    Bigger,BiggerEq,Smaller,SmallerEq,NotEq
}

#[derive(Debug, PartialEq, Eq,Clone, Copy)]
pub enum FilteredDice{
    Simple(Dice),
    Filtered(Dice,Filter,u32)
}

#[derive(Debug, PartialEq, Eq,Clone, Copy)]
pub enum Selector{
    Higher,
    Lower
}

#[derive(Debug, PartialEq, Eq,Clone, Copy)]
pub enum SelectedDice{
    Unchanged(FilteredDice),
    Selected(FilteredDice,Selector,u32)
}

#[derive(Debug,PartialEq, Eq,Clone, Copy)]
pub enum Operation{
    Mul,Div,Add,Sub
}

#[derive(Debug, PartialEq, Eq,Clone)]
pub enum Term{
    Constant(i64),
    DiceThrow(SelectedDice),
    Calculation(Box<Term>,Operation,Box<Term>),
    SubTerm(Box<Term>)
}

#[derive(Debug, PartialEq, Eq,Clone)]
pub enum Expression{
    Simple(Term),
    List(u32,Term)
}
