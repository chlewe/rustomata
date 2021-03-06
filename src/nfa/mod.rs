use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::ops::{AddAssign, Mul, MulAssign};

use num_traits::{One, Zero};

use integeriser::{Integeriser, HashIntegeriser};

use recognisable::*;
use recognisable::automaton::Automaton;
use push_down_automaton::*;

/// Dictonary able to translate a `NFA` back into a `PushDownAutomaton`
#[derive(Debug, PartialEq)]
pub struct Dict<I: Instruction, T: Eq + Hash, W: Eq + Ord>{
    map: HashMap<NFATransition<usize, T, W>, Transition<I, T, W>>,
}

/// `Transition` equivalent for `NFA`
#[derive(Debug, Clone)]
pub struct NFATransition<S: Eq + Hash, T: Eq + Hash, W: Ord + Eq>{
    from_state: S,
    to_state: S,
    word: Vec<T>,
    weight: W,
}

/// Structure encoding an Automaton without storage (i.e. not an `Automaton`).
#[derive(Debug, Clone)]
pub struct NFA<S: Eq + Hash, T: Eq + Hash, W: Eq + Ord>{
    //states: HashSet<S>,
    transitions: HashMap<S, BinaryHeap<NFATransition<S, T, W>>>,
    initial_states: HashSet<S>,
    final_states: HashSet<S>,
}

impl<S: Eq + Hash + Ord + Clone, T: Eq + Hash + Clone + Ord, W: Eq + Ord + One + Clone> NFA<S, T, W>{
    pub fn new(/*states: HashSet<S>,*/ transitions: HashMap<S, BinaryHeap<NFATransition<S, T, W>>>, initial_states: HashSet<S>, final_states: HashSet<S>)-> NFA<S, T, W>{
        NFA{
            //states: states,
            transitions,
            initial_states,
            final_states,
        }
    }

    pub fn recognise(&self, word: &[T]) -> NFARecogniser<S, T, W> {
        let mut init_heap = BinaryHeap::new();
        for i in self.initial_states.clone(){
            let c = Configuration {
                word: word.to_vec(),
                storage: i.clone(),
                weight: W::one(),
            };
            init_heap.push((c, Vec::new()));
        }
        NFARecogniser {
            agenda: init_heap,
            filtered_rules: self.transitions.clone(),
            accepting: self.final_states.clone(),
            //used: HashSet::new(),
        }
    }
}

impl<S: Eq + Clone + Hash, T: Eq + Clone + Hash, W: Ord + Eq + Clone + Mul<Output=W>> NFATransition<S, T, W>{
    pub fn new(from: S, to: S, word: Vec<T>, weight: W)->Self{
        NFATransition{
            from_state: from,
            to_state: to,
            word,
            weight,
        }
    }

    pub fn apply(&self, c: &Configuration<S, T, W>) -> Vec<Configuration<S, T, W>>{
        if !(c.word.starts_with(&self.word[..])) || !(c.storage == self.from_state) {
            return Vec::new()
        }
        let c1 = Configuration{
            word: c.word.clone().split_off(self.word.len()),
            storage: self.to_state.clone(),
            weight: c.weight.clone() * self.weight.clone(),
        };
        vec![c1]
    }
}

impl<S: Eq + Hash, T: Eq + Hash, W: Ord + PartialOrd + Eq> PartialOrd for NFATransition<S, T, W> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.weight.partial_cmp(&other.weight)
    }
}

impl<S: Eq + Hash, T: Eq + Hash, W: Ord + Eq> Ord for NFATransition<S, T, W> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.weight.cmp(&other.weight)
    }
}

impl<S: Eq + Hash, T: Eq + Hash, W: Ord + Eq> PartialEq for NFATransition<S, T, W> {
    fn eq(&self, other: &NFATransition<S, T, W>)-> bool{
        (self.from_state == other.from_state) &
        (self.to_state == other.to_state) &
        (self.word == other.word) &
        (self.weight == other.weight)
    }
}

impl<S: Eq + Hash, T: Eq + Hash, W: Ord + Eq> Hash for NFATransition<S, T, W> {
    fn hash<H: Hasher>(&self, state: &mut H){
        self.from_state.hash(state);
        self.to_state.hash(state);
        self.word.hash(state);
    }
}

impl<S: Eq + Hash, T: Eq + Hash, W: Ord + Eq> Eq for NFATransition<S, T, W> {}

impl<I: Instruction + Clone, T: Eq + Hash + Clone, W: Eq + Clone + Ord> Dict<I, T, W> {
    pub fn new(map: HashMap<NFATransition<usize, T, W>, Transition<I, T, W>>)->Self {
        Dict{
            map,
        }
    }

    pub fn translate(&self, v: Vec<NFATransition<usize, T, W>>)-> Vec<Transition<I, T, W>> {
        let mut outv = Vec::new();
        for t in v{
            match self.map.get(&t){
                Some(t2) => {
                    outv.push(t2.clone());
                },
                None => {
                    return Vec::new();
                },
            }
        }
        outv
    }
}

/// `Recogniser` equivalent for `NFA`
pub struct NFARecogniser<S: Clone + Ord + Hash + Eq, T: Eq + Hash, W: Eq + Ord> {
    agenda: BinaryHeap<(Configuration<S, T, W>, Vec<NFATransition<S, T, W>>)>,
    filtered_rules: HashMap<S, BinaryHeap<NFATransition<S, T, W>>>,
    accepting: HashSet<S>,
    //used: HashSet<Configuration<S, T, W>>,
}

impl<S: Clone + Ord + Hash + Eq, T: Eq + Hash, W: Eq + Ord> NFARecogniser<S, T, W> {
    fn accepts(&self, c: &Configuration<S, T, W>)-> bool{
        self.accepting.contains(&c.storage) && c.word.is_empty()
    }
}

impl<S: Clone + Ord + Hash + Eq, T: Eq + Hash + Clone + Ord, W: One + Mul<Output = W> + Clone + Eq + Ord> Iterator for NFARecogniser<S, T, W> {
    type Item = (Configuration<S, T, W>, Vec<NFATransition<S, T, W>>);

    fn next(&mut self) -> Option<(Configuration<S, T, W>, Vec<NFATransition<S, T, W>>)> {
        while let Some((c, run)) = self.agenda.pop() {
            //self.used.insert(c.clone());
            if let Some(rs) = self.filtered_rules.get(&(c.storage)) {
                for r in rs {
                    let cv = r.apply(&c);
                    for c1 in cv{
                        //if !self.used.contains(&c1){
                        let mut run1 = run.clone();
                        run1.push(r.clone());
                        self.agenda.push((c1, run1))
                        //}
                    }
                }
            }
            if self.accepts(&c) {
                return Some((c, run));
            }
        }

        None
    }
}

/// Creates a `NFA` from a `PushDownAutomaton` including `Dict` to translate it back. Returns `None` when a `Replace` instruction is found
pub fn from_pd<A, T, W>(a: &PushDownAutomaton<A, T, W>)
                        -> Option<(NFA<usize, T, W>, Dict<PushDownInstruction<A>, T, W>)>
    where A: Clone + Hash + Ord + PartialEq,
          T: Clone + Eq + Hash + Ord,
          W: AddAssign + Clone + Copy + Eq + Mul<Output=W> + MulAssign + One + Ord + Zero,
{
    let mut integeriser: HashIntegeriser<PushDown<A>> = HashIntegeriser::new();
    let map: HashMap<NFATransition<usize, T, W>, Transition<PushDownInstruction<A>, T, W>> = HashMap::new();
    let mut to_do = Vec::new();
    let mut states = HashSet::new();
    let mut initial_states = HashSet::new();
    let mut final_states = HashSet::new();

    let transitions = HashMap::new();

    let mut transition_map = HashMap::new();
    for t in a.transitions() {
        let key =
            match t.instruction {
                PushDownInstruction::Replace { ref current_val, .. } => current_val.first().unwrap().clone(),
            };
        transition_map.entry(key).or_insert(Vec::new()).push(t);
    }

    initial_states.insert(integeriser.integerise(a.initial()));
    to_do.push(a.initial());

    while let Some(c) = to_do.pop(){
        let ci = integeriser.integerise(c.clone());
        states.insert(ci);
        if c.is_bottom(){
            final_states.insert(ci);
        }
        if let Some(rs) = transition_map.get(c.current_symbol()) {
            for r in rs{
                match r.instruction{
                    PushDownInstruction::Replace {..} => {
                        return None;
                    }
                }
            }
        }
    }
    Some((NFA::new(/*states,*/ transitions, initial_states, final_states), Dict::new(map)))
}
