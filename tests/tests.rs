pub mod test_logging {
    use log::{Level, LevelFilter, Metadata, Record};
    use std::cell::RefCell;
    use std::sync::Once;

    static INIT: Once = Once::new();

    thread_local! {
        static LOG_CONTENTS: RefCell<Vec<String>> = RefCell::new(Vec::new());
    }

    struct TestLogger;

    impl log::Log for TestLogger {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= Level::Info
        }

        fn log(&self, record: &Record) {
            if self.enabled(record.metadata()) {
                let log_entry = format!("{} - {}", record.level(), record.args());
                LOG_CONTENTS.with(|contents| {
                    contents.borrow_mut().push(log_entry);
                });
            }
        }

        fn flush(&self) {}
    }

    pub fn setup_logger() {
        INIT.call_once(|| {
            log::set_boxed_logger(Box::new(TestLogger))
                .map(|()| log::set_max_level(LevelFilter::Info))
                .unwrap();
        });
    }

    pub fn clear_log_contents() {
        LOG_CONTENTS.with(|contents| {
            contents.borrow_mut().clear();
        });
    }

    pub fn get_log_contents() -> Vec<String> {
        LOG_CONTENTS.with(|contents| contents.borrow().clone())
    }
}

pub mod test_git {
    use git_test::git::{get_repo_root, GitRepository};
    use std::path::Path;
    use tempfile::TempDir;

    pub fn init_git_repo(temp_dir: &Path) {
        std::process::Command::new("git")
            .args(&["init"])
            .current_dir(temp_dir)
            .status()
            .unwrap();
    }

    pub fn setup_test() -> (TempDir, GitRepository) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();
        init_git_repo(repo_path);
        let repo = get_repo_root(repo_path).unwrap();
        (temp_dir, repo)
    }
}

mod test_command_add {
    use anyhow::Result;

    use crate::test_git::setup_test;
    use crate::test_logging::{clear_log_contents, get_log_contents, setup_logger};
    use git_test::commands::add::cmd_add;

    #[test]
    fn test_add_new_test() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test();

        cmd_add(&repo, "default", false, false, "just default")?;

        let command = repo.get_test_command("default")?;
        assert_eq!(command.value(), "just default");

        assert_eq!(
            get_log_contents(),
            vec!["INFO - Changing test 'default' from '<empty>' to 'just default'",]
        );
        Ok(())
    }

    #[test]
    fn test_add_multiple_tests() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test();

        cmd_add(&repo, "default", false, false, "just default")?;
        cmd_add(
            &repo,
            "spotless-formats",
            false,
            false,
            "just spotless formats",
        )?;
        cmd_add(
            &repo,
            "spotless-java-sort-imports",
            false,
            false,
            "just spotless java-sort-imports",
        )?;

        assert_eq!(repo.get_test_command("default")?.value(), "just default");
        assert_eq!(
            repo.get_test_command("spotless-formats")?.value(),
            "just spotless formats"
        );
        assert_eq!(
            repo.get_test_command("spotless-java-sort-imports")?.value(),
            "just spotless java-sort-imports"
        );

        assert_eq!(get_log_contents(), vec![
            "INFO - Changing test 'default' from '<empty>' to 'just default'",
            "INFO - Changing test 'spotless-formats' from '<empty>' to 'just spotless formats'",
            "INFO - Changing test 'spotless-java-sort-imports' from '<empty>' to 'just spotless java-sort-imports'",
        ]);
        Ok(())
    }

    #[test]
    fn test_add_existing_test_no_flags() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test();

        cmd_add(&repo, "default", false, false, "old command")?;
        cmd_add(&repo, "default", false, false, "new command")?;

        assert_eq!(get_log_contents(), vec![
            "INFO - Changing test 'default' from '<empty>' to 'old command'",
            "WARN - Overwriting existing test 'default'. Use --forget to delete stored results or --keep to preserve them.",
            "INFO - Changing test 'default' from 'old command' to 'new command'",
        ]);
        assert_eq!(repo.get_test_command("default")?.value(), "new command");
        Ok(())
    }

    #[test]
    fn test_add_existing_test_with_forget() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test();

        cmd_add(&repo, "default", false, false, "old command")?;
        cmd_add(&repo, "default", true, false, "new command")?;

        assert_eq!(
            get_log_contents(),
            vec![
                "INFO - Changing test 'default' from '<empty>' to 'old command'",
                "INFO - Changing test 'default' from 'old command' to 'new command'",
            ]
        );
        assert_eq!(repo.get_test_command("default")?.value(), "new command");
        Ok(())
    }

    #[test]
    fn test_add_existing_test_with_keep() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test();

        cmd_add(&repo, "default", false, false, "old command")?;
        cmd_add(&repo, "default", false, true, "new command")?;

        assert_eq!(
            get_log_contents(),
            vec![
                "INFO - Changing test 'default' from '<empty>' to 'old command'",
                "INFO - Changing test 'default' from 'old command' to 'new command'"
            ]
        );
        assert_eq!(repo.get_test_command("default")?.value(), "new command");
        Ok(())
    }

    #[test]
    fn test_add_existing_test_with_forget_and_keep() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test();

        cmd_add(&repo, "default", false, false, "old command")?;
        cmd_add(&repo, "default", true, true, "new command")?;

        assert_eq!(
            get_log_contents(),
            vec![
                "INFO - Changing test 'default' from '<empty>' to 'old command'",
                "INFO - Changing test 'default' from 'old command' to 'new command'"
            ]
        );
        assert_eq!(repo.get_test_command("default")?.value(), "new command");
        Ok(())
    }

    #[test]
    fn test_add_existing_test_with_same_command() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test();

        cmd_add(&repo, "default", false, false, "same command")?;
        cmd_add(&repo, "default", false, false, "same command")?;

        assert_eq!(get_log_contents(), vec![
            "INFO - Changing test 'default' from '<empty>' to 'same command'",
            "WARN - Overwriting existing test 'default'. Use --forget to delete stored results or --keep to preserve them.",
            "INFO - Changing test 'default' from 'same command' to 'same command'",
        ]);
        assert_eq!(repo.get_test_command("default")?.value(), "same command");
        Ok(())
    }

    #[test]
    fn test_add_nonexistent_test() {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test();

        let result = repo.get_test_command("nonexistent");
        assert!(result.is_err());

        assert_eq!(get_log_contents(), Vec::<String>::new());
    }
}

mod test_command_list {
    use crate::test_git::setup_test;
    use crate::test_logging::{clear_log_contents, get_log_contents, setup_logger};
    use anyhow::Result;
    use git_test::commands::cmd_list;

    #[test]
    fn test_list_tests() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test();

        repo.set_test_command("default", "just default")?;
        repo.set_config_value("test.default.description", "Default test")?;
        repo.set_test_command("spotless-formats", "just spotless formats")?;
        repo.set_config_value("test.spotless-formats.description", "Spotless formats test")?;
        repo.set_test_command(
            "spotless-java-sort-imports",
            "just spotless java-sort-imports",
        )?;
        repo.set_test_command("empty-command", "")?;

        cmd_list(&repo)?;

        let log_contents = get_log_contents();
        let expected_logs = vec![
            "INFO - default:",
            "INFO -     command = just default",
            "INFO - spotless-formats:",
            "INFO -     command = just spotless formats",
            "INFO - spotless-java-sort-imports:",
            "INFO -     command = just spotless java-sort-imports",
            "INFO - empty-command:",
            "INFO -     command = ",
        ];

        assert_eq!(log_contents, expected_logs);
        Ok(())
    }
}
