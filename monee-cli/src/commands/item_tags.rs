use monee::actions::item_tags;

#[derive(clap::Subcommand)]
pub enum ItemTagsCommand {
    #[command(alias = "c")]
    Create {
        /// Unique identifier
        #[arg(short, long)]
        name: String,
    },

    #[command(alias = "r")]
    Relate {
        /// Unique identifier
        #[arg(short, long)]
        parent: String,

        /// Unique identifier
        #[arg(short, long)]
        child: String,
    },

    #[command(alias = "v")]
    View,

    #[command(alias = "u")]
    Unlink {
        /// Unique identifier
        #[arg(short, long)]
        parent: String,

        /// Unique identifier
        #[arg(short, long)]
        child: String,
    },
}

pub fn handle(command: ItemTagsCommand) -> miette::Result<()> {
    match command {
        ItemTagsCommand::Create { name } => create(name),
        ItemTagsCommand::Relate { parent, child } => relate(parent, child),
        ItemTagsCommand::View => view(),
        ItemTagsCommand::Unlink { parent, child } => unlink(parent, child),
    }
}

pub fn create(name: String) -> miette::Result<()> {
    crate::tasks::block_single(async {
        let db = crate::tasks::use_db().await?;

        let tag = monee_core::item_tag::ItemTag { name: name.clone() };
        match item_tags::create::run(&db, tag).await {
            Ok(()) => {
                println!("Tag `{}` created", name);
                Ok(())
            }
            Err(item_tags::create::Error::Database(db_err)) => monee::log::database(db_err),
            Err(item_tags::create::Error::AlreadyExists) => {
                let diagnostic = miette::diagnostic!(
                    severity = miette::Severity::Error,
                    code = "item_tag::AlreadyExists",
                    "Item tag with name `{}` already exists",
                    name
                );

                Err(diagnostic.into())
            }
        }
    })
}

async fn recover_tags(
    db: &monee::database::Connection,
    parent: String,
    child: String,
) -> miette::Result<(
    monee_core::item_tag::ItemTagId,
    monee_core::item_tag::ItemTagId,
)> {
    fn diagnostic_not_found(tag_name: String) -> miette::Report {
        miette::diagnostic!(
            severity = miette::Severity::Error,
            code = "item_tag::NotFound",
            "Item tag with id `{}` not found",
            tag_name
        )
        .into()
    }

    match tokio::try_join!(
        item_tags::get::run(db, parent.clone()),
        item_tags::get::run(db, child.clone())
    ) {
        Ok((Some(parent), Some(child))) => Ok((parent, child)),
        Err(why) => monee::log::database(why),
        Ok((None, _)) => Err(diagnostic_not_found(child)),
        Ok((_, None)) => Err(diagnostic_not_found(parent)),
    }
}

pub fn relate(parent: String, child: String) -> miette::Result<()> {
    crate::tasks::block_multi(async {
        let db = crate::tasks::use_db().await?;

        let (parent_tag, child_tag) = recover_tags(&db, parent.clone(), child.clone()).await?;

        match item_tags::relate::run(&db, parent_tag, child_tag).await {
            Ok(()) => {
                println!("Tag `{}` now contains `{}`", parent, child);
                Ok(())
            }
            Err(item_tags::relate::Error::Database(db_err)) => monee::log::database(db_err),
            Err(item_tags::relate::Error::NotFound(tag_id)) => {
                let diagnostic = miette::diagnostic!(
                    severity = miette::Severity::Error,
                    code = "item_tag::NotFound",
                    "Item tag with id `{}` not found",
                    tag_id
                );

                Err(diagnostic.into())
            }
            Err(item_tags::relate::Error::CyclicRelation) => {
                let diagnostic = miette::diagnostic!(
                    severity = miette::Severity::Error,
                    code = "item_tag::CyclicRelation",
                    "Item tag `{}` already contains `{}`",
                    child_tag,
                    parent_tag
                );

                Err(diagnostic.into())
            }

            Err(item_tags::relate::Error::AlreadyContains) => {
                let diagnostic = miette::diagnostic!(
                    severity = miette::Severity::Error,
                    code = "item_tag::AlreadyContains",
                    "Item tag `{}` already contains `{}`",
                    parent,
                    child
                );

                Err(diagnostic.into())
            }
        }
    })
}

pub fn view() -> miette::Result<()> {
    let result: miette::Result<_> = crate::tasks::block_single(async {
        let db = crate::tasks::use_db().await?;

        match item_tags::view::run(&db).await {
            Ok(tags) => Ok(tags),
            Err(why) => monee::log::database(why),
        }
    });

    let tags = result?;

    for (tag, children) in tags {
        print!("{}", tag.name);

        if !children.is_empty() {
            print!(": {}", children.join(", "));
        }

        println!();
    }

    Ok(())
}

pub fn unlink(parent: String, child: String) -> miette::Result<()> {
    crate::tasks::block_multi(async {
        let db = crate::tasks::use_db().await?;
        let (parent_tag, child_tag) = recover_tags(&db, parent.clone(), child.clone()).await?;

        match item_tags::unlink::run(&db, parent_tag, child_tag).await {
            Ok(()) => {
                println!("Tag `{}` no longer contains `{}`", parent, child);
                Ok(())
            }
            Err(item_tags::unlink::Error::Database(db_err)) => monee::log::database(db_err),
            Err(item_tags::unlink::Error::NotFound(tag_id)) => {
                let diagnostic = miette::diagnostic!(
                    severity = miette::Severity::Error,
                    code = "item_tag::NotFound",
                    "Item tag with id `{}` not found",
                    tag_id
                );

                Err(diagnostic.into())
            }
        }
    })
}
