extern crate num_traits;

use std::collections::{BinaryHeap, HashMap};
use std::convert::From;
use std::fmt::{self, Debug, Display};
use std::hash::Hash;
use std::ops::Mul;
use std::rc::Rc;
use std::vec::Vec;

use num_traits::One;

use integeriser::{HashIntegeriser, Integeriser};
use recognisable::{Configuration, Item, Recognisable, Transition};
use recognisable::automaton::{Automaton, recognise, recognise_beam};
use util::integerisable::{Integerisable1, Integerisable2};
use util::push_down::Pushdown;

mod from_pmcfg;
mod from_str;
mod tree_stack;
mod tree_stack_instruction;

pub mod cli;

pub use self::from_pmcfg::*;
pub use self::tree_stack::*;
pub use self::tree_stack_instruction::*;


type TransitionMap<A, T, W> = HashMap<A, BinaryHeap<Transition<TreeStackInstruction<A>, T, W>>>;

/// Automaton with storage type `TreeStack<A>`, terminals of type `T` and weights of type `W`.
#[derive(Debug, Clone)]
pub struct TreeStackAutomaton<A, T, W>
    where A: Clone + Hash + Ord,
          T: Eq + Hash,
          W: Ord,
{
    a_integeriser: HashIntegeriser<A>,
    t_integeriser: HashIntegeriser<T>,
    transitions: Rc<TransitionMap<usize, usize, W>>,
    initial: TreeStack<usize>,
}


impl<A, T, W> TreeStackAutomaton<A, T, W>
    where A: Clone + Eq + Hash + Ord,
          T: Clone + Eq + Hash,
          W: Clone + Ord,
{
    pub fn new(transitions: Vec<Transition<TreeStackInstruction<A>, T, W>>,
               initial: TreeStack<A>)
               -> TreeStackAutomaton<A, T, W>
    {
        let mut a_inter = HashIntegeriser::new();
        let mut t_inter = HashIntegeriser::new();
        let init: TreeStack<usize> = initial.integerise(&mut a_inter);
        let mut transition_map: TransitionMap<usize, usize, W>  = HashMap::new();

        for t in transitions.iter().map(|t| t.integerise(&mut t_inter, &mut a_inter)) {
            let a =
                match t.instruction {
                    TreeStackInstruction::Up     { ref current_val, .. }
                    | TreeStackInstruction::Push { ref current_val, .. }
                    | TreeStackInstruction::Down { ref current_val, .. } => current_val.clone()
                };

            if !transition_map.contains_key(&a) {
                transition_map.insert(a.clone(), BinaryHeap::new());
                ()
            }

            transition_map.get_mut(&a).unwrap().push(t);
        }

        TreeStackAutomaton {
            a_integeriser: a_inter,
            t_integeriser: t_inter,
            transitions: Rc::new(transition_map),
            initial: init,
        }
    }

    pub fn list_transitions<'a>(&'a self)
                                -> Box<Iterator<Item=Transition<TreeStackInstruction<A>, T, W>> + 'a>
    {
        Box::new(
            self.transitions.values()
                .flat_map(
                    move |h| h.iter()
                        .map(move |t| Transition::un_integerise(t,
                                                                &self.t_integeriser,
                                                                &self.a_integeriser))
                )
        )
    }

    pub fn initial(&self) -> TreeStack<A> { // TODO change type, maybe remove
        TreeStack::un_integerise(&self.initial, &self.a_integeriser).clone()
    }

    pub fn transitions(&self) -> TransitionMap<A, T, W> { // TODO change type, maybe remove
        let mut result = HashMap::new();

        for k in self.transitions.keys() {
            let mut vec = BinaryHeap::new();
            for t in &self.transitions[k] {
                vec.push(Transition::un_integerise(t, &self.t_integeriser, &self.a_integeriser));
            }
            result.insert(self.a_integeriser.find_value(*k).unwrap().clone(), vec);
        }

        result
    }
}


impl<A, T, W> Recognisable<T, W> for TreeStackAutomaton<A, T, W>
    where A: Ord + PartialEq + Debug + Clone + Hash,
          T: Clone + Debug + Eq + Hash + Ord,
          W: One + Mul<Output=W> + Clone + Copy + Debug + Eq + Ord
{
    type Parse = Item<TreeStack<A>, TreeStackInstruction<A>, T, W>;

    fn recognise<'a>(&'a self, word: Vec<T>) -> Box<Iterator<Item=Self::Parse> + 'a> {
        recognise(self, word)
    }

    fn recognise_beam_search<'a>(&'a self, beam: usize, word: Vec<T>) -> Box<Iterator<Item=Self::Parse> + 'a> {
        recognise_beam(self, beam, word)
    }
}


impl<A, T, W> Automaton<T, W> for TreeStackAutomaton<A, T, W>
    where A: Clone + Debug + Eq + Hash + Ord,
          T: Clone + Debug + Eq + Hash + PartialOrd,
          W: Clone + Copy + Debug + Eq + Mul<Output=W> + One + Ord,
{
    type I = TreeStackInstruction<A>;
    type IInt = TreeStackInstruction<usize>;
    type TInt = usize;
    type Key = usize;

    fn extract_key(c: &Configuration<TreeStack<usize>, usize, W>) -> &usize {
        match *c {
            Configuration { ref storage, .. } => storage.current_symbol(),
        }
    }

    fn is_terminal(c: &Configuration<TreeStack<usize>, usize, W>) -> bool {
        c.word.is_empty() && c.storage.is_at_bottom()
    }

    fn item_map(&self, i: &Item<TreeStack<usize>, TreeStackInstruction<usize>, usize, W>)
                -> Item<TreeStack<A>, TreeStackInstruction<A>, T, W> {
        match *i {
            (Configuration { ref word, ref storage, weight }, ref pd) => {
                let pd_vec: Vec<_>
                    = pd.clone().into();
                let pd_unint: Vec<_>
                    = pd_vec.iter().map(
                        |t| Integerisable2::un_integerise(t,
                                                          &self.t_integeriser,
                                                          &self.a_integeriser))
                            .collect();
                (
                    Configuration {
                        word: word
                            .iter()
                            .map(|t| self.t_integeriser.find_value(*t).unwrap().clone())
                            .collect(),
                        storage: Integerisable1::un_integerise(storage, &self.a_integeriser),
                        weight: weight,
                    },
                    Pushdown::from(pd_unint.as_slice())
                )
            }
        }
    }

    fn transitions<'a>(&'a self) -> Box<Iterator<Item=Transition<TreeStackInstruction<A>, T, W>> + 'a> {
        self.list_transitions()
    }

    fn transition_map(&self) -> Rc<TransitionMap<usize, usize, W>> {
        self.transitions.clone()
    }

    fn initial(&self) -> TreeStack<usize> {
        self.initial.clone()
    }

    fn terminal_to_int(&self, t: &T) -> usize {
        self.t_integeriser.find_key(t).unwrap()
    }
}


impl<A, T, W> Display for TreeStackAutomaton<A, T, W>
    where A: Ord + PartialEq + Debug + Clone + Hash + Display,
          T: Clone + Debug + Eq + Hash,
          W: One + Mul<Output=W> + Clone + Copy + Debug + Eq + Ord + Display
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut formatted_transitions = String::new();
        for t in self.list_transitions() {
            formatted_transitions.push_str(&t.to_string());
            formatted_transitions.push_str("\n");
        }
        write!(f, "initial: {}\n\n{}", self.initial.current_symbol(), formatted_transitions)
    }
}
