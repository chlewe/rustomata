use std::marker::PhantomData;

use automata;
pub use approximation::*;
use util::equivalence_classes::*;

pub use tree_stack::*;
pub use push_down::*;

// relabel function for configurations and states
pub trait Relabel<N1, N2, O>{
    fn relabel(&self, &EquivalenceClass<N1, N2>) -> O;
}

//Strategy Element for Relabel
pub struct RlbElement<A, N1, N2>{
    pub dummy: PhantomData<A>,
    pub mapping: EquivalenceClass<N1, N2>
}

impl <A1 : Ord + PartialEq + Debug + Clone + Hash + Relabel<N1, N2, A2>,
      A2:  Ord + PartialEq + Debug + Clone + Hash,
      N1: Clone + Eq + Hash,
      N2: Clone + Eq + Hash,
      T: Eq + Clone +Hash,
      W: Ord + Eq + Clone + Add<Output=W> + Mul<Output = W> + Div<Output = W> + Zero + One> ApproximationStrategy<PushDown<A1>, PushDown<A2>,
        automata::Transition<PushDown<A1>, PushDownInstruction<A1>, T, W>,
        automata::Transition<PushDown<A2>, PushDownInstruction<A2>, T, W>>
      for RlbElement<PushDown<A1>, N1, N2>{
    fn approximate_initial(&self, a : PushDown<A1>)-> PushDown<A2>{
        a.relabel(&self.mapping)
    }

    fn approximate_transition(&self, t :  automata::Transition<PushDown<A1>, PushDownInstruction<A1>, T, W>) ->
        automata::Transition<PushDown<A2>, PushDownInstruction<A2>, T, W>{
        match t.instruction{
            PushDownInstruction::Replace {ref current_val, ref new_val} => {
                let mut stc = Vec::new();
                let mut stn = Vec::new();
                for nt in current_val{
                    stc.push(nt.relabel(&self.mapping));
                }
                for nt in new_val{
                    stn.push(nt.relabel(&self.mapping));
                }
                automata::Transition {
                    _dummy: PhantomData,
                    word: t.word.clone(),
                    weight: t.weight.clone(),
                    instruction: PushDownInstruction::Replace {
                        current_val: stc.clone(),
                        new_val: stn.clone(),
                    }
                }
            },
            PushDownInstruction::ReplaceK {ref current_val, ref new_val, ref limit} => {
                let mut stc = Vec::new();
                let mut stn = Vec::new();
                for nt in current_val{
                    stc.push(nt.relabel(&self.mapping));
                }
                for nt in new_val{
                    stn.push(nt.relabel(&self.mapping));
                }
                automata::Transition {
                    _dummy: PhantomData,
                    word: t.word.clone(),
                    weight: t.weight.clone(),
                    instruction: PushDownInstruction::ReplaceK {
                        current_val: stc.clone(),
                        new_val: stn.clone(),
                        limit: limit.clone(),
                    }
                }
            },
        }
    }
}