use crate::{ast::symbols::Primitive, data_types::TimedValue, ts_utils::parsing::ParseBuiltin};

pub trait ToPrimitve {
    fn to_primitive() -> Primitive;
}

impl ToPrimitve for usize {
    fn to_primitive() -> Primitive {
        Primitive::Integer
    }
}

impl ToPrimitve for f32 {
    fn to_primitive() -> Primitive {
        Primitive::Float
    }
}

impl ToPrimitve for bool {
    fn to_primitive() -> Primitive {
        Primitive::Boolean
    }
}

impl ToPrimitve for String {
    fn to_primitive() -> Primitive {
        Primitive::String
    }
}

impl<T: ToPrimitve> ToPrimitve for TimedValue<T> {
    fn to_primitive() -> Primitive {
        T::to_primitive()
    }
}

impl<T: ParseBuiltin> ToPrimitve for T {
    fn to_primitive() -> Primitive {
        Self::KIND
    }
}
