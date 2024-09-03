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
            metadata.level() <= Level::Debug
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
                .map(|()| log::set_max_level(LevelFilter::Debug))
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
    use std::path::PathBuf;
    use std::process::Command;
    use tempfile::TempDir;

    pub fn init_git_repo(temp_dir: &PathBuf) {
        Command::new("git")
            .args(&["init"])
            .current_dir(temp_dir)
            .status()
            .unwrap();
    }

    pub fn setup_test() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();
        init_git_repo(&repo_path);
        (temp_dir, repo_path)
    }
}

mod test_command_add {
    use anyhow::Result;

    use crate::test_git::setup_test;
    use crate::test_logging::{clear_log_contents, get_log_contents, setup_logger};
    use git_test::commands::add::cmd_add;
    use git_test::git;

    #[test]
    fn test_add_new_test() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo_path) = setup_test();

        cmd_add(&repo_path, "default", false, false, "just default")?;

        let command = git::get_test_command(&repo_path, "default")?;
        assert_eq!(command, "just default");

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
        let (_temp_dir, repo_path) = setup_test();

        cmd_add(&repo_path, "default", false, false, "just default")?;
        cmd_add(
            &repo_path,
            "spotless-formats",
            false,
            false,
            "just spotless formats",
        )?;
        cmd_add(
            &repo_path,
            "spotless-java-sort-imports",
            false,
            false,
            "just spotless java-sort-imports",
        )?;

        assert_eq!(
            git::get_test_command(&repo_path, "default")?,
            "just default"
        );
        assert_eq!(
            git::get_test_command(&repo_path, "spotless-formats")?,
            "just spotless formats"
        );
        assert_eq!(
            git::get_test_command(&repo_path, "spotless-java-sort-imports")?,
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
        let (_temp_dir, repo_path) = setup_test();

        cmd_add(&repo_path, "default", false, false, "old command")?;
        cmd_add(&repo_path, "default", false, false, "new command")?;

        assert_eq!(get_log_contents(), vec![
            "INFO - Changing test 'default' from '<empty>' to 'old command'",
            "WARN - Overwriting existing test 'default'. Use --forget to delete stored results or --keep to preserve them.",
            "INFO - Changing test 'default' from 'old command' to 'new command'",
        ]);
        assert_eq!(git::get_test_command(&repo_path, "default")?, "new command");
        Ok(())
    }

    #[test]
    fn test_add_existing_test_with_forget() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo_path) = setup_test();

        cmd_add(&repo_path, "default", false, false, "old command")?;
        cmd_add(&repo_path, "default", true, false, "new command")?;

        assert_eq!(
            get_log_contents(),
            vec![
                "INFO - Changing test 'default' from '<empty>' to 'old command'",
                "INFO - Changing test 'default' from 'old command' to 'new command'",
            ]
        );
        assert_eq!(git::get_test_command(&repo_path, "default")?, "new command");
        Ok(())
    }

    #[test]
    fn test_add_existing_test_with_keep() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo_path) = setup_test();

        cmd_add(&repo_path, "default", false, false, "old command")?;
        cmd_add(&repo_path, "default", false, true, "new command")?;

        assert_eq!(
            get_log_contents(),
            vec![
                "INFO - Changing test 'default' from '<empty>' to 'old command'",
                "INFO - Changing test 'default' from 'old command' to 'new command'"
            ]
        );
        assert_eq!(git::get_test_command(&repo_path, "default")?, "new command");
        Ok(())
    }

    #[test]
    fn test_add_existing_test_with_forget_and_keep() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo_path) = setup_test();

        cmd_add(&repo_path, "default", false, false, "old command")?;
        cmd_add(&repo_path, "default", true, true, "new command")?;

        assert_eq!(
            get_log_contents(),
            vec![
                "INFO - Changing test 'default' from '<empty>' to 'old command'",
                "INFO - Changing test 'default' from 'old command' to 'new command'"
            ]
        );
        assert_eq!(git::get_test_command(&repo_path, "default")?, "new command");
        Ok(())
    }

    #[test]
    fn test_add_existing_test_with_same_command() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo_path) = setup_test();

        cmd_add(&repo_path, "default", false, false, "same command")?;
        cmd_add(&repo_path, "default", false, false, "same command")?;

        assert_eq!(get_log_contents(), vec![
            "INFO - Changing test 'default' from '<empty>' to 'same command'",
            "WARN - Overwriting existing test 'default'. Use --forget to delete stored results or --keep to preserve them.",
            "INFO - Changing test 'default' from 'same command' to 'same command'",
        ]);
        assert_eq!(
            git::get_test_command(&repo_path, "default")?,
            "same command"
        );
        Ok(())
    }

    #[test]
    fn test_add_nonexistent_test() {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo_path) = setup_test();

        let result = git::get_test_command(&repo_path, "nonexistent");
        assert!(result.is_err());

        assert_eq!(get_log_contents(), Vec::<String>::new());
    }
}
