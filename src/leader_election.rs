const C_LE: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LEState {
    le_count: usize,
    coin_count: usize,
    leader_done: bool,
    is_leader: bool,
    error: bool,
}

impl LEState {
    pub fn new(num_phases: usize) -> LEState {
        LEState {
            le_count: C_LE * num_phases,
            coin_count: num_phases,
            leader_done: false,
            is_leader: false,
            error: false,
        }
    }

    pub fn is_leader(self) -> bool {
        self.is_leader
    }

    pub fn is_error(self) -> bool {
        self.error
    }
}

pub fn le_interact(mut u: LEState, v: LEState, v_coin: bool) -> (LEState, LEState) {
    assert!(!u.error && !v.error, "must catch errors; {:?} {:?}", u, v);
    assert!(!u.is_leader && !v.is_leader, "must transition leaders");

    if !v_coin {
        u.leader_done = true;
    }
    if !u.leader_done && u.coin_count > 0 {
        u.coin_count -= 1;
    }

    if u.coin_count == 0 {
        u.is_leader = true;
        u.leader_done = true;
    }

    u.le_count -= 1;

    if u.le_count == 0 {
        u.error = true;
        return (u, v);
    }

    (u, v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_agent_only_flips_coin() {
        let num_phases = 3;

        let u = LEState::new(num_phases);
        let mut v = LEState::new(num_phases);

        let fresh = LEState::new(num_phases);

        (_, v) = le_interact(u, v, true);

        assert_eq!(v, fresh);
    }

    #[test]
    fn elect_leader_correctly() {
        let num_phases = 3;

        let mut u = LEState::new(num_phases);
        let fresh = LEState::new(num_phases);

        (u, _) = le_interact(u, fresh, true);

        assert_eq!(u.le_count, fresh.le_count - 1);
        assert_eq!(u.coin_count, 2);
        assert!(!u.is_leader);
        assert!(!u.leader_done);
        assert!(!u.error);

        (u, _) = le_interact(u, fresh, true);

        assert_eq!(u.le_count, fresh.le_count - 2);
        assert_eq!(u.coin_count, 1);
        assert!(!u.is_leader);
        assert!(!u.leader_done);
        assert!(!u.error);

        (u, _) = le_interact(u, fresh, true);

        assert_eq!(u.le_count, fresh.le_count - 3);
        assert_eq!(u.coin_count, 0);
        assert!(u.is_leader);
        assert!(u.leader_done);
        assert!(!u.error);
    }

    #[test]
    fn elect_follower_correctly() {
        let num_phases = 3;

        let mut u = LEState::new(num_phases);
        let fresh = LEState::new(num_phases);

        (u, _) = le_interact(u, fresh, false);

        assert_eq!(u.le_count, fresh.le_count - 1);
        assert_eq!(u.coin_count, 3);
        assert!(!u.is_leader);
        assert!(u.leader_done);
        assert!(!u.error);
    }

    #[test]
    fn error_on_timeout() {
        let num_phases = 3;

        let mut u = LEState::new(num_phases);
        let fresh = LEState::new(num_phases);

        let timer_val = u.le_count;

        for i in 0..timer_val - 1 {
            (u, _) = le_interact(u, fresh, false);
            assert_eq!(u.le_count, fresh.le_count - (i + 1));
            assert_eq!(u.coin_count, 3);
            assert!(!u.is_leader);
            assert!(u.leader_done);
            assert!(!u.error);
        }

        (u, _) = le_interact(u, fresh, false);
        assert!(u.error);
    }
}
