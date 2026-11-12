/// Marcador de value object: igualdad por valor, inmutable.
pub trait ValueObject: Clone + Eq {}

/// Marcador de raíz de agregado.
pub trait AggregateRoot {
    type Id: ValueObject;

    fn id(&self) -> &Self::Id;
}
