// Defines the algebraic structure for state convergence (Join-Semilattice)
pub trait Lattice: Clone + PartialEq + PartialOrd {
    /// Computes the Least Upper Bound (LUB) of two states.
    /// Corresponds to `s1 ⊔ s2` in Basic.lean.
    fn join(&self, other: &Self) -> Self;

    /// Computes the Greatest Lower Bound (GLB) of two states.
    /// Corresponds to `s1 ⊓ s2` in Basic.lean.
    fn meet(&self, other: &Self) -> Self;
}

pub trait OrderBot: Lattice {
    /// The bottom element of the lattice.
    /// Corresponds to `⊥` in Basic.lean.
    fn bot() -> Self;
}

impl Lattice for u64 {
    fn join(&self, other: &Self) -> Self {
        std::cmp::max(*self, *other)
    }
    fn meet(&self, other: &Self) -> Self {
        std::cmp::min(*self, *other)
    }
}

impl OrderBot for u64 {
    fn bot() -> Self {
        0
    }
}

impl Lattice for bool {
    fn join(&self, other: &Self) -> Self {
        *self || *other
    }
    fn meet(&self, other: &Self) -> Self {
        *self && *other
    }
}

impl OrderBot for bool {
    fn bot() -> Self {
        false
    }
}
