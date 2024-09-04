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
                let log_entry = format!("{}", record.args());
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

    pub fn set_color_enabled(enabled: bool) {
        colored::control::set_override(enabled);
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

    pub async fn setup_test() -> (TempDir, GitRepository) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path();
        init_git_repo(repo_path);
        let repo = get_repo_root(repo_path).await.unwrap();
        (temp_dir, repo)
    }
}

pub mod test_cli {
    use clap::{ColorChoice, Parser};
    use git_test::cli::{Cli, Commands};

    #[test]
    fn test_color_default_is_auto() {
        let cli = Cli::try_parse_from(&["git-test", "list"]).unwrap();
        assert_eq!(cli.color, ColorChoice::Auto);
    }

    #[test]
    fn test_color_always() {
        let cli = Cli::try_parse_from(&["git-test", "--color", "always", "list"]).unwrap();
        assert_eq!(cli.color, ColorChoice::Always);
    }

    #[test]
    fn test_color_never() {
        let cli = Cli::try_parse_from(&["git-test", "--color", "never", "list"]).unwrap();
        assert_eq!(cli.color, ColorChoice::Never);
    }

    #[test]
    fn test_invalid_color_choice() {
        let result = Cli::try_parse_from(&["git-test", "--color", "invalid", "list"]);
        assert!(result.is_err());
    }

    #[test]
    fn test_subcommand_parsing() {
        let cli = Cli::try_parse_from(&["git-test", "list"]).unwrap();
        assert!(matches!(cli.command, Commands::List));

        let cli =
            Cli::try_parse_from(&["git-test", "add", "--test", "default", "command"]).unwrap();
        assert!(matches!(cli.command, Commands::Add(_)));

        let cli = Cli::try_parse_from(&["git-test", "run", "--test", "default"]).unwrap();
        assert!(matches!(cli.command, Commands::Run(_)));
    }
}
mod test_command_add {
    use anyhow::Result;

    use crate::test_git::setup_test;
    use crate::test_logging::{
        clear_log_contents, get_log_contents, set_color_enabled, setup_logger,
    };
    use git_test::commands::add::cmd_add;

    #[tokio::test]
    async fn test_add_new_test() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test().await;

        cmd_add(&repo, "default", false, false, "just default").await?;

        let command = repo.get_test_command("default").await?;
        assert_eq!(command.value(), "just default");

        assert_eq!(
            get_log_contents(),
            vec!["Changing test 'default' from '<empty>' to 'just default'",]
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_add_multiple_tests() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test().await;

        cmd_add(&repo, "default", false, false, "just default").await?;
        cmd_add(
            &repo,
            "spotless-formats",
            false,
            false,
            "just spotless formats",
        )
        .await?;
        cmd_add(
            &repo,
            "spotless-java-sort-imports",
            false,
            false,
            "just spotless java-sort-imports",
        )
        .await?;

        assert_eq!(
            repo.get_test_command("default").await?.value(),
            "just default"
        );
        assert_eq!(
            repo.get_test_command("spotless-formats").await?.value(),
            "just spotless formats"
        );
        assert_eq!(
            repo.get_test_command("spotless-java-sort-imports")
                .await?
                .value(),
            "just spotless java-sort-imports"
        );

        assert_eq!(get_log_contents(), vec![
            "Changing test 'default' from '<empty>' to 'just default'",
            "Changing test 'spotless-formats' from '<empty>' to 'just spotless formats'",
            "Changing test 'spotless-java-sort-imports' from '<empty>' to 'just spotless java-sort-imports'",
        ]);
        Ok(())
    }

    #[tokio::test]
    async fn test_add_existing_test_no_flags() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test().await;

        cmd_add(&repo, "default", false, false, "old command").await?;
        cmd_add(&repo, "default", false, false, "new command").await?;

        assert_eq!(get_log_contents(), vec![
            "Changing test 'default' from '<empty>' to 'old command'",
            "Overwriting existing test 'default'. Use --forget to delete stored results or --keep to preserve them.",
            "Changing test 'default' from 'old command' to 'new command'",
        ]);
        assert_eq!(
            repo.get_test_command("default").await?.value(),
            "new command"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_add_existing_test_with_forget() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test().await;

        cmd_add(&repo, "default", false, false, "old command").await?;
        cmd_add(&repo, "default", true, false, "new command").await?;

        assert_eq!(
            get_log_contents(),
            vec![
                "Changing test 'default' from '<empty>' to 'old command'",
                "Changing test 'default' from 'old command' to 'new command'",
            ]
        );
        assert_eq!(
            repo.get_test_command("default").await?.value(),
            "new command"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_add_existing_test_with_keep() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test().await;

        cmd_add(&repo, "default", false, false, "old command").await?;
        cmd_add(&repo, "default", false, true, "new command").await?;

        assert_eq!(
            get_log_contents(),
            vec![
                "Changing test 'default' from '<empty>' to 'old command'",
                "Changing test 'default' from 'old command' to 'new command'"
            ]
        );
        assert_eq!(
            repo.get_test_command("default").await?.value(),
            "new command"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_add_existing_test_with_forget_and_keep() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test().await;

        cmd_add(&repo, "default", false, false, "old command").await?;
        cmd_add(&repo, "default", true, true, "new command").await?;

        assert_eq!(
            get_log_contents(),
            vec![
                "Changing test 'default' from '<empty>' to 'old command'",
                "Changing test 'default' from 'old command' to 'new command'"
            ]
        );
        assert_eq!(
            repo.get_test_command("default").await?.value(),
            "new command"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_add_existing_test_with_same_command() -> Result<()> {
        setup_logger();
        clear_log_contents();
        set_color_enabled(false);
        let (_temp_dir, repo) = setup_test().await;

        cmd_add(&repo, "default", false, false, "same command").await?;
        cmd_add(&repo, "default", false, false, "same command").await?;

        assert_eq!(get_log_contents(), vec![
            "Changing test 'default' from '<empty>' to 'same command'",
            "Overwriting existing test 'default'. Use --forget to delete stored results or --keep to preserve them.",
            "Changing test 'default' from 'same command' to 'same command'",
        ]);
        assert_eq!(
            repo.get_test_command("default").await?.value(),
            "same command"
        );
        Ok(())
    }

    #[tokio::test]
    async fn test_add_nonexistent_test() {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test().await;

        let result = repo.get_test_command("nonexistent");
        assert!(result.await.is_err());

        assert_eq!(get_log_contents(), Vec::<String>::new());
    }
}

mod test_command_list {
    use crate::test_git::setup_test;
    use crate::test_logging::{clear_log_contents, get_log_contents, setup_logger};
    use anyhow::Result;
    use git_test::commands::cmd_list;

    #[tokio::test]
    async fn test_list_tests() -> Result<()> {
        setup_logger();
        clear_log_contents();
        let (_temp_dir, repo) = setup_test().await;

        repo.set_test_command("default", "just default").await?;
        repo.set_config_value("test.default.description", "Default test")
            .await?;
        repo.set_test_command("spotless-formats", "just spotless formats")
            .await?;
        repo.set_config_value("test.spotless-formats.description", "Spotless formats test")
            .await?;
        repo.set_test_command(
            "spotless-java-sort-imports",
            "just spotless java-sort-imports",
        )
        .await?;
        repo.set_test_command("empty-command", "").await?;

        cmd_list(&repo).await?;

        let log_contents = get_log_contents();
        let expected_logs = vec![
            "default:",
            "    command = just default",
            "spotless-formats:",
            "    command = just spotless formats",
            "spotless-java-sort-imports:",
            "    command = just spotless java-sort-imports",
            "empty-command:",
            "    command = ",
        ];

        assert_eq!(log_contents, expected_logs);
        Ok(())
    }
}
