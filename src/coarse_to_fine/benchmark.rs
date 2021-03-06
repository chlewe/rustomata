use time::PreciseTime;
use std::io::Write;
use std::fs::File;
use rand::{SeedableRng, StdRng};
use rand::distributions::{IndependentSample, Range};
use std::collections::HashSet;

use pmcfg::*;
use coarse_to_fine;
use nfa::*;

use log_domain::LogDomain;

use approximation::Approximation;
use approximation::tts::TTSElement;
use approximation::relabel::{RlbElement, EquivalenceClass};
use approximation::ptk::PDTopKElement;
use recognisable::Recognisable;

use tree_stack_automaton::{TreeStackAutomaton, PosState};

/// Test a multitude of combinations for coarse-to-fine parsing and takes their times.
/// Results in extra file `benchmark-results.txt`.
/// Does not test words that are longer than twenty.
pub fn benchmark(grammar: PMCFG<String, String, LogDomain<f64>>, eq: EquivalenceClass<String, String>, ptk_size: usize, limit: usize, limit1: usize, limit2: usize, limit3: usize, corpus: &str, check: usize, no_nfa: bool){
    // File that contains the results
    let mut file = File::create("benchmark-results.txt").unwrap();
    let _ = write!(&mut file, "Benchmarking results \n\n");
    let w = 14;

    eprintln!("Start Initialisation");

    // creates the initial approximation strategies
    let ap_start = PreciseTime::now();
    let tts = TTSElement::new();
    let ap_1 = PreciseTime::now();
    let f = |ps: &PosState<PMCFGRule<_, _, _>>| ps.map(|r| r.map_nonterminals(|nt| eq.project(nt)));
    let rlb = RlbElement::new(&f);
    let ap_2 = PreciseTime::now();
    let ptk = PDTopKElement::new(ptk_size);
    let ap_end = PreciseTime::now();

    // creates all automata that are to be used
    eprintln!("Automaton");
    let at_start = PreciseTime::now();
    let automaton = TreeStackAutomaton::from(grammar);
    let at_1 = PreciseTime::now();
    eprintln!("TTS");
    let (app1, ntts) = automaton.approximation(tts).unwrap();
    let at_2 = PreciseTime::now();
    eprintln!("RLB");
    let (app2, nrlb) = app1.approximation(rlb).unwrap();
    let at_3 = PreciseTime::now();
    eprintln!("PTK");
    let (app3, nptk) = app2.approximation(ptk).unwrap();
    let at_4 = PreciseTime::now();
    eprintln!("NFA");
    let nfa_s = if no_nfa { None } else { from_pd(&app3) };
    let at_end = PreciseTime::now();

    // save times for initial startup
    let _ = write!(&mut file, "Construction TTS: {}\nConstruction RLB: {}\nConstruction PTK: {}\n\nGeneration Automata: {}\nApproximation TTS: {}\nApproximation RLB: {}\nApproximation PTK: {}\nNFAs: {}\n\nRecognition times:\n",
                   ap_start.to(ap_1),
                   ap_1.to(ap_2),
                   ap_2.to(ap_end),
                   at_start.to(at_1),
                   at_1.to(at_2),
                   at_2.to(at_3),
                   at_3.to(at_4),
                   at_4.to(at_end)
    );
    let _ = write!(&mut file, "\n{0: <width$} | {1: <width$} | {2: <width$} | {3: <width$} | {4: <width$} | {5: <width$} | {6: <width$} \n",
                   "Word", "Normal", "1-Layer", "2-Layers", "3-Layers", "3-Layers + NFA", "id. output", width = w);
    let mut outercount = 0;
    eprintln!("Start Test");

    let seed: &[_] = &[1, 2, 3, 4];
    let mut rng: StdRng = SeedableRng::from_seed(seed);
    let range = Range::new(0, corpus.lines().count());

    // chooses a number of nonempty sentences to compute
    let mut to_check = Vec::new();
    let mut in_to_check = HashSet::new();

    while check > to_check.len(){
        let i = range.ind_sample(&mut rng);
        let sentence = match corpus.lines().nth(i){
            Some(x) => {x},
            None => {continue;},
        };
        // creates the word
        let word: Vec<String> = sentence.split_whitespace().map(|x| x.to_string()).collect();
        // pushes element in to check if word can be generated and sample not already in to_check
        if !word.is_empty() && !in_to_check.contains(&i) && word.len() <= 25 {
            to_check.push((sentence, word));
            in_to_check.insert(i);
        }
    }

    if no_nfa {
        for (sentence, word) in to_check {
            eprintln!("{}:", sentence);
            let mut r_set: HashSet<Vec<_>> = HashSet::new();
            let mut same = true;
            //No approximation
            eprintln!("no Approximation");
            let p1_start = PreciseTime::now();
            for parse in automaton.recognise(word.clone()).take(limit) {
                //eprintln!("{}", Run::new(parse.translate().1));
                eprintln!("Found run");
                r_set.insert(parse.1.into());
            }
            let p1_end = PreciseTime::now();

            eprintln!("1-Layer");
            //TTS
            let p2_start = PreciseTime::now();
            let mut c = 0;
            for parse3 in app1.recognise(word.clone()).take(limit1) {
                let s3 = coarse_to_fine::ctf_level(parse3.1.into(), &ntts, &automaton);
                for parse4 in s3 {
                    //eprintln!("{}", Run::new(parse4.translate().1));
                    eprintln!("Found run");
                    if !r_set.contains(&parse4.1){
                        same = false;
                    }
                    c += 1;
                    if c>=limit{
                        break
                    }
                }
                if c >= limit{
                    break;
                }
            }
            let p2_end = PreciseTime::now();

            eprintln!("2-Layers");
            //TTS -> RLB
            let p3_start = PreciseTime::now();
            let mut c = 0;
            let mut c1 = 0;
            for parse2 in app2.recognise(word.clone()).take(limit2) {
                let s2 = coarse_to_fine::ctf_level(parse2.1.into(), &nrlb, &app1);
                for parse3 in s2{
                    let s3 = coarse_to_fine::ctf_level(parse3.1.into(), &ntts, &automaton);
                    for parse4 in s3{
                        //eprintln!("{}", Run::new(parse4.translate().1));
                        eprintln!("Found run");
                        if !r_set.contains(&parse4.1){
                            same = false;
                        }
                        c += 1;
                        if c>=limit{
                            break
                        }
                    }
                    c1 += 1;
                    if c>=limit||c1>=limit1{
                        break;
                    }
                }
                if c1>=limit1||c>=limit{
                    break;
                }
            }
            let p3_end = PreciseTime::now();

            eprintln!("3-Layers");
            // TTS -> RLB -> PTK
            let p4_start = PreciseTime::now();
            let mut c = 0;
            let mut c1 = 0;
            let mut c2 = 0;
            for parse1 in app3.recognise(word.clone()).take(limit3) {
                let s1 = coarse_to_fine::ctf_level(parse1.1.into(), &nptk, &app2);
                for parse2 in s1{
                    let s2 = coarse_to_fine::ctf_level(parse2.1.into(), &nrlb, &app1);
                    for parse3 in s2{
                        let s3 = coarse_to_fine::ctf_level(parse3.1.into(), &ntts, &automaton);
                        for parse4 in s3{
                            //eprintln!("{}", Run::new(parse4.translate().1));
                            eprintln!("Found run");
                            if !r_set.contains(&parse4.1){
                                same = false;
                            }
                            c += 1;
                            if c>=limit{
                                break
                            }
                        }
                        c1 += 1;
                        if c >= limit || c1 >= limit1 {
                            break;
                        }
                    }
                    c2 += 1;
                    if c2 >= limit2 || c1 >= limit1 || c >= limit {
                        break;
                    }
                }
                if c2 >= limit2 || c1 >= limit1 || c >= limit {
                    break;
                }
            }
            let p4_end = PreciseTime::now();

            eprintln!("3-Layers + NFA");
            //TTS -> RLB -> PTK -> TO_NFA
            let p5_start = PreciseTime::now();

            let p5_end = PreciseTime::now();
            outercount += 1;

            //save results and times for this sentence
            let _ = write!(&mut file, "\n{0: <width$} | {1: <width$} | {2: <width$} | {3: <width$} | {4: <width$} | {5: <width$} | {6: <width$} \n",
                           outercount, p1_start.to(p1_end), p2_start.to(p2_end), p3_start.to(p3_end), p4_start.to(p4_end), p5_start.to(p5_end), same, width = w);


        }
    } else {
        let (nfa, nfa_dict) = nfa_s.unwrap();
        for (sentence, word) in to_check {
            let mut r_set: HashSet<Vec<_>> = HashSet::new();
            let mut same = true;
            eprintln!("{}:", sentence);

            //No approximation
            eprintln!("no Approximation");
            let p1_start = PreciseTime::now();
            for parse in automaton.recognise(word.clone()).take(limit) {
                //eprintln!("{}", Run::new(parse.translate().1));
                eprintln!("Found run");
                r_set.insert(parse.1.into());
            }
            let p1_end = PreciseTime::now();

            eprintln!("1-Layer");
            //TTS
            let p2_start = PreciseTime::now();
            let mut c = 0;
            for parse3 in app1.recognise(word.clone()).take(limit1) {
                let s3 = coarse_to_fine::ctf_level(parse3.1.into(), &ntts, &automaton);
                for parse4 in s3{
                    //eprintln!("{}", Run::new(parse4.translate().1));
                    eprintln!("Found run");
                    if !r_set.contains(&parse4.1){
                        same = false;
                    }
                    c += 1;
                    if c>=limit{
                        break
                    }
                }
                if c>=limit{
                    break;
                }
            }
            let p2_end = PreciseTime::now();

            eprintln!("2-Layers");
            //TTS -> RLB
            let p3_start = PreciseTime::now();
            let mut c = 0;
            let mut c1 = 0;
            for parse2 in app2.recognise(word.clone()).take(limit2) {
                let s2 = coarse_to_fine::ctf_level(parse2.1.into(), &nrlb, &app1);
                for parse3 in s2{
                    let s3 = coarse_to_fine::ctf_level(parse3.1.into(), &ntts, &automaton);
                    for parse4 in s3{
                        //eprintln!("{}", Run::new(parse4.translate().1));
                        eprintln!("Found run");
                        if !r_set.contains(&parse4.1){
                            same = false;
                        }
                        c += 1;
                        if c>=limit{
                            break
                        }
                    }
                    c1 += 1;
                    if c>=limit||c1>=limit1{
                        break;
                    }
                }
                if c1>=limit1||c>=limit{
                    break;
                }
            }
            let p3_end = PreciseTime::now();

            eprintln!("3-Layers");
            //TTS -> RLB -> PTK
            let p4_start = PreciseTime::now();
            let mut c = 0;
            let mut c1 = 0;
            let mut c2 = 0;
            for parse1 in app3.recognise(word.clone()).take(limit3) {
                let s1 = coarse_to_fine::ctf_level(parse1.1.into(), &nptk, &app2);
                for parse2 in s1{
                    let s2 = coarse_to_fine::ctf_level(parse2.1.into(), &nrlb, &app1);
                    for parse3 in s2{
                        let s3 = coarse_to_fine::ctf_level(parse3.1.into(), &ntts, &automaton);
                        for parse4 in s3{
                            //eprintln!("{}", Run::new(parse4.translate().1));
                            eprintln!("Found run");
                            if !r_set.contains(&parse4.1){
                                same = false;
                            }
                            c += 1;
                            if c>=limit{
                                break
                            }
                        }
                        c1 += 1;
                        if c>=limit||c1>=limit1{
                            break;
                        }
                    }
                    c2 += 1;
                    if c2>=limit2||c1>=limit1||c>=limit{
                        break;
                    }
                }
                if c2>=limit2||c1>=limit1||c>=limit{
                    break;
                }
            }
            let p4_end = PreciseTime::now();

            eprintln!("3-Layers + NFA");
            //TTS -> RLB -> PTK -> TO_NFA
            let p5_start = PreciseTime::now();
            let mut c = 0;
            let mut c1 = 0;
            let mut c2 = 0;
            for parsenfa in nfa.recognise(&word).take(limit3) {
                let parse1 = nfa_dict.translate(parsenfa.1);
                let s1 = coarse_to_fine::ctf_level(parse1, &nptk, &app2);
                for parse2 in s1{
                    let s2 = coarse_to_fine::ctf_level(parse2.1.into(), &nrlb, &app1);
                    for parse3 in s2{
                        let s3 = coarse_to_fine::ctf_level(parse3.1.into(), &ntts, &automaton);
                        for parse4 in s3{
                            //eprintln!("{}", Run::new(parse4.translate().1));
                            eprintln!("Found run");
                            if !r_set.contains(&parse4.1){
                                same = false;
                            }
                            c += 1;
                            if c>=limit{
                                break
                            }
                        }
                        c1 += 1;
                        if c>=limit||c1>=limit1{
                            break;
                        }
                    }
                    c2 += 1;
                    if c2>=limit2||c1>=limit1||c>=limit{
                        break;
                    }
                }
                if c2>=limit2||c1>=limit1||c>=limit{
                    break;
                }
            }
            let p5_end = PreciseTime::now();
            outercount += 1;

            //save results and times for this sentence
            let _ = write!(&mut file, "\n{0: <width$} | {1: <width$} | {2: <width$} | {3: <width$} | {4: <width$} | {5: <width$} | {6: <width$} \n",
                           outercount, p1_start.to(p1_end), p2_start.to(p2_end), p3_start.to(p3_end), p4_start.to(p4_end), p5_start.to(p5_end), same, width = w);
        }
    }
    //tests different combinations and takes individual times, randomized
}
