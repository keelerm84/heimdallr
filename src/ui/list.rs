use crate::application::list_instances::Handler;
use anyhow::{anyhow, Result};
use itertools::Itertools;
use prettytable::{cell, format, row, Table};

pub async fn list(handler: Handler<'_>) -> Result<()> {
    let running_instances = handler.list().await?;

    if running_instances.is_empty() {
        return Err(anyhow!("No instances were found"));
    }

    let format = format::FormatBuilder::new()
        .column_separator('│')
        .borders('│')
        .separators(
            &[format::LinePosition::Title],
            format::LineSeparator::new('─', '┼', '├', '┤'),
        )
        .padding(1, 1)
        .build();
    let mut table = Table::new();
    table.set_format(format);
    table.set_titles(row![Fgb->"Environment", Fgb->"Name", Fgb->"Instance Id"]);

    let environment_count = running_instances.keys().count();
    for (i, env) in running_instances.keys().sorted().enumerate() {
        let mut instances = running_instances.get(env).unwrap().to_owned();
        instances.sort_by(|lhs, rhs| lhs.0.partial_cmp(&rhs.0).unwrap());

        for instance in instances {
            table.add_row(row![Fbb->env, Fyb->instance.0, Fcb->instance.1]);
        }

        if i + 1 != environment_count {
            table.add_row(row![]);
        }
    }

    table.printstd();

    Ok(())
}
