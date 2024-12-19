mod leader_election;

use leader_election::{LEState, le_interact};

use rand::prelude::*;
use rayon::prelude::*;
use indicatif::ParallelProgressIterator;

const C_WAIT: usize = 2;
const C_LIVE: usize = 4;
const C_RESET: usize = 4;
const C_DELAY: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Rank(usize),
    LE(LEState),
    Waiting(usize, usize),
    Phase(usize, usize),
    Propagating(usize),
    Dormant(usize),
}

impl State {
    fn is_electing(self) -> bool {
        match self {
            LE(_) => true,
            _ => false,
        }
    }

    fn is_ranked(self) -> bool {
        match self {
            Rank(_) => true,
            _ => false,
        }
    }

    fn is_computing(self) -> bool {
        match self {
            Propagating(_) | Dormant(_) => false,
            _ => true,
        }
    }

    fn is_main(self) -> bool {
        match self {
            Rank(_) | Phase(..) | Waiting(..) => true,
            _ => false,
        }
    }

    fn alive_count(self) -> Option<usize> {
        match self {
            Phase(_, result) | Waiting(_, result) => Some(result),
            _ => None,
        }
    }

    fn with_reset_alive_count(self, num_phases: usize) -> State {
        self.with_new_alive_count(C_LIVE * num_phases)
    }

    fn with_new_alive_count(self, new_count: usize) -> State {
        assert!(self.alive_count().is_some());

        match self {
            Phase(phase, _) => Phase(phase, new_count),
            Waiting(wait_count, _) => Waiting(wait_count, new_count),
            _ => unreachable!(),
        }
    }
}

use State::*;

#[derive(Debug, Clone)]
struct Protocol {
    num_phases: usize,
    n: usize,
    states: Vec<State>,
    coins: Vec<Option<bool>>,
    num_ranked: usize,
    range_starts: Vec<usize>,
    range_ends: Vec<usize>,
    range_lengths: Vec<usize>,
}

impl Protocol {
    fn new(n: usize) -> Protocol {
        let phases = num_phases(n);
        let mut range_starts = vec![usize::MAX];
        let mut range_ends = vec![usize::MAX];
        let mut range_lengths = vec![usize::MAX];
        let mut rem = n;
        for _ in 1..=phases {
            let end = rem;
            let start = (rem >> 1) + (rem & 1) + 1;
            let length = end - start + 1;
            assert!(start + length - 1 == rem);
            range_ends.push(end);
            range_starts.push(start);
            range_lengths.push(length);
            rem -= length;
        }

        Protocol {
            num_phases: num_phases(n),
            n,
            coins: vec![Some(true); n],
            states: vec![LE(LEState::new(phases)); n],
            num_ranked: 0,
            range_starts,
            range_ends,
            range_lengths,
        }
    }

    fn new_from_leader_election(n: usize) -> Protocol {
        let mut result = Protocol::new(n);

        result.states[0] = Rank(1);
        result.coins[0] = None;
        result.num_ranked = 1;

        result
    }

    fn new_completely_ranked_with_dupe(n: usize) -> Protocol {
        let mut result = Protocol::new(n);

        for i in 0..n {
            result.states[i] = Rank(i+1);
            result.coins[i] = None;
            result.num_ranked += 1;
        }
        result.states[0] = Phase(result.num_phases, C_LIVE * result.num_phases);
        result.coins[0] = Some(false);
        result.num_ranked -= 1;

        result
    }

    fn update(&mut self, i: usize, j: usize, _t: usize) {
        let ranks_before = self.states[i].is_ranked() as usize + self.states[j].is_ranked() as usize;
        
        (self.states[i], self.states[j]) = self.interact(self.states[i], self.states[j], self.coins[j]);
        if let Some(coin) = self.coins[j] {
            self.coins[j] = Some(!coin);
        }

        for k in [i, j] {
            if let Rank(_) = self.states[k] {
                self.coins[k] = None;
            } else if self.coins[k].is_none() {
                self.coins[k] = Some(false);
            }
        }

        let ranks_after = self.states[i].is_ranked() as usize + self.states[j].is_ranked() as usize;
        self.num_ranked = (self.num_ranked - ranks_before) + ranks_after;
    }

    fn interact(&self, mut u: State, mut v: State, v_coin: Option<bool>) -> (State, State) {
        (u, v) = self.propagate_reset(u, v);

        // perform core leader election
        if let (LE(mut u_le), LE(mut v_le)) = (u, v) {
            (u_le, v_le) = le_interact(u_le, v_le, v_coin.unwrap());

            if u_le.is_error() {
                // println!("LE resetting");
                u = Propagating(C_RESET * self.num_phases)
            } else if u_le.is_leader() {
                // println!("leader elected");
                u = Waiting(C_WAIT * self.num_phases, C_LIVE * self.num_phases);
            } else {
                u = LE(u_le);
            }

            if v_le.is_error() {
                // println!("LE resetting");
                v = Propagating(C_RESET * self.num_phases);
            } else if v_le.is_leader() {
                //println!("leader elected");
                v = Waiting(C_WAIT * self.num_phases, C_LIVE * self.num_phases);
            } else {
                v = LE(v_le);
            }
        }

        // propagate epidemic from electing agents to main agents
        if u.is_electing() && v.is_main() {
            u = Phase(1, C_LIVE * self.num_phases)
        }
        if v.is_electing() && u.is_main() {
            v = Phase(1, C_LIVE * self.num_phases)
        }

        // run main protocol
        if u.is_main() && v.is_main() {
            self.ranking_interact(u, v, v_coin)
        } else {
            (u, v)
        }
    }

    fn ranking_interact(&self, mut u: State, mut v: State, v_coin: Option<bool>) -> (State, State) {
        assert!(!u.is_electing() && !v.is_electing());

        if let (Rank(u_rank), Rank(v_rank)) = (u, v) {
            if u_rank == v_rank {
                // two agents of same rank => reset
                return (Propagating(C_RESET * self.num_phases), v);
            }
        }

        if let (Some(u_live), Some(v_live)) = (u.alive_count(), v.alive_count()) {
            assert!(u_live >= 1 && v_live >= 1);
            let ctr = std::cmp::max(u_live, v_live) - 1;
            if ctr == 0 {
                // aliveness counter reached 0 => reset
                return (Propagating(C_RESET * self.num_phases), v);
            } else {
                // still alive => update counters
                u = u.with_new_alive_count(ctr);
                v = v.with_new_alive_count(ctr);
            }
        }

        if let (Rank(u_rank), Some(v_lc)) = (u, v.alive_count()) {
            // encountering agent with rank n-1 or n also decreases liveness counter
            if u_rank + 1 >= self.n {
                if v_lc > 1 {
                    v = v.with_new_alive_count(v_lc - 1);
                } else {
                    // TODO: this is not like written in the paper: there, the check of going
                    // into resetting is missing
                    return (u, Propagating(C_RESET * self.num_phases));
                }
            }
        }

        if v_coin == Some(false) {
            assert!(v.alive_count().is_some(), "{:?}", v);
            v = match u {
                Waiting(_, _) => v.with_reset_alive_count(self.num_phases),
                Rank(u_rank) => {
                    if let Phase(phase, _) = v {
                        if u_rank <= self.range_lengths[phase] {
                            v.with_reset_alive_count(self.num_phases)
                        } else {
                            v
                        }
                    } else {
                        v
                    }
                },
                _ => v,
            };
            return (u, v);
        }

        let (phase, v_lc) = match v {
            Phase(k, lc) => (k, lc),
            _ => { return (u, v); },
        };

        assert!(v_coin == Some(true));

        match u {
            Rank(rank) => {
                if rank <= self.range_lengths[phase] {
                    v = Rank(self.range_starts[phase] + (rank - 1));
                    if rank < self.range_lengths[phase] {
                        u = Rank(rank + 1);
                    } else if phase < self.num_phases as usize {
                        u = Waiting(C_WAIT * self.num_phases, C_LIVE * self.num_phases);
                    }
                }
                if rank == self.range_ends[phase] {
                    let new_phase = phase + 1;
                    v = if new_phase <= self.num_phases {
                        Phase(phase + 1, C_LIVE * self.num_phases)
                    } else {
                        Propagating(C_RESET * self.num_phases)
                    }
                }
            },
            Phase(u_phase,u_lc) => {
                let max_phase = std::cmp::max(u_phase, phase);
                u = Phase(max_phase, u_lc);
                v = Phase(max_phase, v_lc);
            },
            Waiting(wait_count,_) => {
                assert!(wait_count > 0);
                if wait_count == 1 {
                    u = Rank(1);
                } else {
                    u = Waiting(wait_count - 1, C_LIVE * self.num_phases);
                }
            }
            _ => unreachable!(),
        }

        (u, v)
    }

    fn propagate_reset(&self, mut u: State, mut v: State) -> (State, State) {
        if let Propagating(u_rc) = u {
            assert!(u_rc > 0);
            if v.is_computing() {
                // v = Dormant(C_DELAY * self.num_phases);
                v = Propagating(u_rc);
            }

            if let Dormant(_) = v {
                u = if u_rc <= 1 {
                    Dormant(C_DELAY * num_phases(self.n))
                } else {
                    Propagating(u_rc - 1)
                };
            } else if let Propagating(v_rc) = v {
                assert!(u_rc >= 1 && v_rc >= 1);
                let new_rc = std::cmp::max(1, std::cmp::max(u_rc, v_rc)) - 1;
                if new_rc == 0 {
                    u = Dormant(C_DELAY * self.num_phases);
                    v = Dormant(C_DELAY * self.num_phases);
                } else {
                    u = Propagating(new_rc);
                    v = Propagating(new_rc);
                }
            }
        }

        if let Dormant(u_dc) = u {
            assert!(u_dc >= 1);
            let new_dc = u_dc - 1;
            if new_dc == 0 {
                u = LE(LEState::new(num_phases(self.n)));
            } else {
                u = Dormant(new_dc);
            }
        }

        if let Dormant(v_dc) = v {
            assert!(v_dc >= 1);
            let new_dc = v_dc - 1;
            if new_dc == 0 {
                v = LE(LEState::new(num_phases(self.n)));
            } else {
                v = Dormant(new_dc);
            }
        }

        (u, v)
    }
}

fn num_phases(n: usize) -> usize {
    assert!(n > 0);
    (usize::BITS - (n - 1).leading_zeros()) as usize
}


fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args[1] == "geom" {
        for j in [7, 8, 9, 10, 11, 12, 13] {
            let n = 2usize.pow(j) as usize;
            eprintln!("run {n} starting");

            let steps = 100;
            (0..steps).into_par_iter().progress_count(steps as u64).for_each(|_| {
                let mut rng = rand::thread_rng();

                let mut protocol = Protocol::new_from_leader_election(n);

                let mut t = 0;
                let mut f = 0.5;
                while (protocol.num_ranked as f64) < (n as f64 * 15.0 / 16.0) {
                    t += 1;
                    let i = rng.gen_range(0..n) as usize;
                    let mut j = rng.gen_range(0..n-1) as usize;
                    if j >= i {
                        j += 1;
                    }
                    
                    protocol.update(i, j, t);
                    while protocol.num_ranked as f64 >= (n as f64 * f) {
                        f = 0.5 * (f + 1.0);
                        println!("{n},{t},{}", protocol.num_ranked);
                    }
                }
            });
        }
    } else if args[1] == "dupe" {
        let n: usize = args[2].parse().unwrap();
        let mut rng = rand::thread_rng();

        let mut protocol = Protocol::new_completely_ranked_with_dupe(n);

        let mut t = 0;
        let mut t_0 = 0;
        while protocol.num_ranked < n {
            t += 1;
            let i = rng.gen_range(0..n) as usize;
            let mut j = rng.gen_range(0..n-1) as usize;
            if j >= i {
                j += 1;
            }
            
            let nr = protocol.num_ranked;
            protocol.update(i, j, t);

            if protocol.num_ranked < n-1 && t_0 == 0 {
                t_0 = t;
            }

            if protocol.num_ranked != nr || t % n == 0 {
                let mut sum = 0;
                let mut count = 0;
                for s in &protocol.states {
                    if let Phase(k, _) = s {
                        sum += k;
                        count += 1;
                    }
                }

                let avg_phase = if count == 0 { 0.0f64 } else { sum as f64 / count as f64 };
                println!("{t},{},{avg_phase}", protocol.num_ranked);
            }
        }
        for i in 0..n-1 {
            for j in i+1..n {
                assert!(protocol.states[i] != protocol.states[j]);
            }
        }
        eprintln!("{t_0},{t},{}", t_0 as f64 / t as f64);
    } else {
        panic!("unknown argument! use 'geom' or 'dupe'!");
    }
}
