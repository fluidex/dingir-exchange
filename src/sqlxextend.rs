pub enum SqlResultExt {
    Done,
    Issue((i32, &'static str)),
}

/* traits which a struct for sql-querying should be implied: TablseSchema and BindQueryArg */
pub trait TableSchemas: 'static {
    fn table_name() -> &'static str;

    /* with to be opted-out by marco later ... */
    //this indicate how many arguments your struct will bind to in the sql statement
    const ARGN: i32;
    //this incidate if these is some value should be "default", mark them by their
    //actual position, start form 1
    //for example: in VALUE (DEFAULT, $1, $2, DEFAULT, $3) should indicate the
    //default value position as [1,4]
    fn default_argsn() -> Vec<i32> {
        vec![]
    }
}

pub trait BindQueryArg<'a, DB>: Sized
where
    DB: sqlx::Database,
{
    //PgArguments sealed its add method as pub(crate) so we can not use the concerete type
    //directly (or apply the more cumbersome "as" statement ...)
    fn bind_args<'g, 'q: 'g>(&'q self, arg: &mut impl sqlx::Arguments<'g, Database = DB>)
    where
        'a: 'q;
}

pub trait CommonSQLQuery<DB: sqlx::Database>: 'static {
    fn sql_statement<T: TableSchemas>() -> String;
}

pub trait CommonSQLQueryWithBind<DB>: CommonSQLQuery<DB>
where
    DB: sqlx::Database,
{
    type DefaultArg: for<'r> sqlx::Arguments<'r, Database = DB> + for<'r> sqlx::IntoArguments<'r, DB>;
}

pub trait FinalQuery {
    fn query_final<T: sqlx::Done>(res: Result<T, sqlx::Error>) -> Result<SqlResultExt, sqlx::Error>;
}

#[tonic::async_trait]
pub trait CommonSqlxAction<DB>: CommonSQLQueryWithBind<DB> + FinalQuery
where
    DB: sqlx::Database,
{
    async fn sql_query<'c, 'a, Q, C>(qr: Q, conn: C) -> Result<SqlResultExt, sqlx::Error>
    where
        C: sqlx::Executor<'c, Database = DB>,
        Q: TableSchemas + BindQueryArg<'a, DB> + Send,
    {
        let mut arg: Self::DefaultArg = Default::default();
        qr.bind_args(&mut arg);

        let ret = sqlx::query_with(&Self::sql_statement::<Q>(), arg).execute(conn).await;
        Self::query_final(ret)
    }
}

#[tonic::async_trait]
pub trait SqlxAction<'a, QT, DB>: TableSchemas + BindQueryArg<'a, DB> + Sized
where
    QT: CommonSQLQueryWithBind<DB> + FinalQuery,
    DB: sqlx::Database,
{
    async fn sql_query<'c, C>(&'a self, conn: C) -> Result<SqlResultExt, sqlx::Error>
    where
        C: sqlx::Executor<'c, Database = DB>,
    {
        let mut arg: QT::DefaultArg = Default::default();
        self.bind_args(&mut arg);

        let sqlstate = QT::sql_statement::<Self>();
        let ret = sqlx::query_with(&sqlstate, arg).execute(conn).await;
        QT::query_final(ret)
    }
}

/* -------- Define for common sql query, now only insert ------------------ */

pub struct InsertTable {}

impl FinalQuery for InsertTable {
    fn query_final<T: sqlx::Done>(res: Result<T, sqlx::Error>) -> Result<SqlResultExt, sqlx::Error> {
        let maydone = res?;
        if maydone.rows_affected() != 1 {
            return Ok(SqlResultExt::Issue((0, "Insert no line")));
        }
        Ok(SqlResultExt::Done)
    }
}

/* ------- Implement for our default db (postgresql) ---------------------- */

struct IterHelper<T1, T2>(T1, T2, (i32, Option<i32>));

impl<T1, T2> Iterator for IterHelper<T1, T2>
where
    T1: Iterator<Item = i32>,
    T2: Iterator<Item = i32>,
{
    type Item = Option<i32>;
    fn next(&mut self) -> Option<Self::Item> {
        self.2 .0 += 1;
        let ret = match self.2 .1 {
            Some(i) => {
                if i > self.2 .0 {
                    self.0.next()
                } else {
                    self.2 .1 = self.1.next();
                    return Some(None);
                }
            }
            None => self.0.next(),
        };
        match ret {
            Some(i) => Some(Some(i)),
            None => None,
        }
    }
}

fn expand_argsn(inp: (i32, Vec<i32>)) -> Vec<Option<i32>> {
    let t1 = 1..(inp.0 + 1);
    let mut t2 = inp.1.into_iter();
    let init = t2.next();
    IterHelper(t1, t2, (0, init)).collect()
}

impl<U> CommonSQLQueryWithBind<sqlx::Postgres> for U
where
    U: CommonSQLQuery<sqlx::Postgres>,
{
    type DefaultArg = sqlx::postgres::PgArguments;
}

impl CommonSQLQuery<sqlx::Postgres> for InsertTable {
    fn sql_statement<T: TableSchemas>() -> String {
        //not good, sql statements can be cached or formed by marco in advance
        //fix it later ...
        format!(
            "INSERT INTO {} VALUES ({})",
            T::table_name(),
            expand_argsn((T::ARGN, T::default_argsn()))
                .iter()
                .map(|i| match i {
                    Some(i) => format!("${}", i),
                    None => String::from("DEFAULT"),
                })
                .fold(String::new(), |acc, s| {
                    if acc.is_empty() {
                        s
                    } else {
                        acc + "," + &s
                    }
                })
        )
    }
}

#[cfg(test)]
mod tests {

    fn gen_insert_str(i: (i32, Vec<i32>)) -> String {
        super::expand_argsn(i)
            .iter()
            .map(|i| match i {
                Some(i) => format!("${}", i),
                None => String::from("DEFAULT"),
            })
            .fold(String::new(), |acc, s| if acc.len() == 0 { s } else { acc + "," + &s })
    }

    #[test]
    fn table_schema_args() {
        assert_eq!(gen_insert_str((4, vec![])), "$1,$2,$3,$4");
        assert_eq!(gen_insert_str((4, vec![3])), "$1,$2,DEFAULT,$3,$4");
        assert_eq!(gen_insert_str((4, vec![1])), "DEFAULT,$1,$2,$3,$4");
        assert_eq!(gen_insert_str((4, vec![1, 6, 7])), "DEFAULT,$1,$2,$3,$4,DEFAULT,DEFAULT");
        assert_eq!(gen_insert_str((1, vec![1, 2, 3])), "DEFAULT,DEFAULT,DEFAULT,$1");
        assert_eq!(gen_insert_str((1, vec![1, 2, 4])), "DEFAULT,DEFAULT,$1,DEFAULT");
    }
}
