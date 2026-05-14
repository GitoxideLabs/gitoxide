mod push {
    use gix_refspec::{Instruction, instruction};

    #[test]
    fn all_matching_branches() {
        assert_eq!(
            Instruction::Push(instruction::Push::AllMatchingBranches {
                allow_non_fast_forward: false
            })
            .to_bstring(),
            ":"
        );
        assert_eq!(
            Instruction::Push(instruction::Push::AllMatchingBranches {
                allow_non_fast_forward: true
            })
            .to_bstring(),
            "+:"
        );
    }

    #[test]
    fn delete() {
        assert_eq!(
            Instruction::Push(instruction::Push::Delete {
                ref_or_pattern: "for-deletion".into(),
            })
            .to_bstring(),
            ":for-deletion"
        );
    }

    #[test]
    fn matching() {
        assert_eq!(
            Instruction::Push(instruction::Push::Matching {
                src: "from".into(),
                dst: "to".into(),
                allow_non_fast_forward: false
            })
            .to_bstring(),
            "from:to"
        );
        assert_eq!(
            Instruction::Push(instruction::Push::Matching {
                src: "from".into(),
                dst: "to".into(),
                allow_non_fast_forward: true
            })
            .to_bstring(),
            "+from:to"
        );
    }
}

mod fetch {
    use gix_refspec::{Instruction, instruction};
    #[test]
    fn only() {
        assert_eq!(
            Instruction::Fetch(instruction::Fetch::Only {
                src: "refs/heads/main".into(),
            })
            .to_bstring(),
            "refs/heads/main"
        );
    }

    #[test]
    fn exclude() {
        assert_eq!(
            Instruction::Fetch(instruction::Fetch::Exclude { src: "excluded".into() }).to_bstring(),
            "^excluded"
        );
    }

    #[test]
    fn and_update() {
        assert_eq!(
            Instruction::Fetch(instruction::Fetch::AndUpdate {
                src: "from".into(),
                dst: "to".into(),
                allow_non_fast_forward: false
            })
            .to_bstring(),
            "from:to"
        );
        assert_eq!(
            Instruction::Fetch(instruction::Fetch::AndUpdate {
                src: "from".into(),
                dst: "to".into(),
                allow_non_fast_forward: true
            })
            .to_bstring(),
            "+from:to"
        );
    }
}
