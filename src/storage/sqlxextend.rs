use std::future::Future;
use std::pin::Pin;

pub enum SqlResultExt {
    QueryResult,
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

pub trait BindQueryArg<'a, DB>
where
    DB: sqlx::Database,
{
    //PgArguments sealed its add method as pub(crate) so we can not use the concerete type
    //directly (or apply the more cumbersome "as" statement ...)
    fn bind_args<'g, 'q: 'g>(&'q self, arg: &mut impl sqlx::Arguments<'g, Database = DB>)
    where
        'a: 'q;
}

pub trait CommonSQLQuery<T: ?Sized, DB: sqlx::Database>: 'static + Sized {
    fn sql_statement() -> String;
    fn sql_statement_rt(_t: &T) -> String {
        Self::sql_statement()
    }
}

pub trait CommonSQLQueryWithBind: sqlx::Database {
    type DefaultArg: for<'r> sqlx::Arguments<'r, Database = Self> + for<'r> sqlx::IntoArguments<'r, Self>;
}

pub trait FinalQuery<DB>
where
    DB: sqlx::Database,
{
    fn query_final(res: Result<<DB as sqlx::Database>::QueryResult, sqlx::Error>) -> Result<SqlResultExt, sqlx::Error>;
}

pub trait SqlxAction<'a, QT, DB>: BindQueryArg<'a, DB>
where
    QT: CommonSQLQuery<Self, DB> + FinalQuery<DB>,
    DB: CommonSQLQueryWithBind,
{
    fn sql_query<'c, 'e, C>(&'a self, conn: C) -> Pin<Box<dyn Future<Output = Result<SqlResultExt, sqlx::Error>> + 'e + Send>>
    where
        C: sqlx::Executor<'c, Database = DB> + 'c,
        'a: 'e,
        'c: 'e,
    {
        let mut arg: <DB as CommonSQLQueryWithBind>::DefaultArg = Default::default();
        self.bind_args(&mut arg);
        let sql_stat = <QT as CommonSQLQuery<Self, DB>>::sql_statement_rt(self);

        Box::pin(async move {
            let ret = sqlx::query_with(&sql_stat, arg).execute(conn).await;
            QT::query_final(ret)
        })
    }
}

pub trait CommonSqlxAction<DB>: FinalQuery<DB>
where
    DB: CommonSQLQueryWithBind,
{
    fn sql_query<'c, 'a, 'e, Q: ?Sized, C>(
        qr: &'a Q,
        conn: C,
    ) -> Pin<Box<dyn Future<Output = Result<SqlResultExt, sqlx::Error>> + 'e + Send>>
    where
        C: sqlx::Executor<'c, Database = DB> + 'c,
        Self: CommonSQLQuery<Q, DB>,
        Q: SqlxAction<'a, Self, DB>,
        'a: 'e,
        'c: 'e,
    {
        SqlxAction::<'a, Self, DB>::sql_query(qr, conn)
    }
}

impl<U: FinalQuery<DB>, DB: CommonSQLQueryWithBind> CommonSqlxAction<DB> for U {}

/* ------- Implement for our default db (postgresql) ---------------------- */

impl CommonSQLQueryWithBind for sqlx::Postgres {
    type DefaultArg = sqlx::postgres::PgArguments;
}

/* -------- Define for common sql query, insert and insertbatch ------------------ */

pub struct InsertTable {}

impl FinalQuery<sqlx::Postgres> for InsertTable {
    fn query_final(res: Result<<sqlx::Postgres as sqlx::Database>::QueryResult, sqlx::Error>) -> Result<SqlResultExt, sqlx::Error> {
        //omit duplicate error
        match res {
            Err(sqlx::Error::Database(dberr)) => {
                if let Some(code) = dberr.code() {
                    if code == "23505" {
                        return Ok(SqlResultExt::Issue((0, "Insert no line")));
                    }
                }
                Err(sqlx::Error::Database(dberr))
            }
            Err(any) => Err(any),
            Ok(done) => {
                if done.rows_affected() != 1 {
                    Ok(SqlResultExt::Issue((0, "Insert no line")))
                } else {
                    Ok(SqlResultExt::QueryResult)
                }
            }
        }
    }
}

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
        ret.map(Some)
    }
}

fn expand_argsn_gen(inp1: (i32, i32), inp2: Vec<i32>) -> Vec<Option<i32>> {
    let t1 = inp1.0..inp1.1;
    let mut t2 = inp2.into_iter();
    let init = t2.next();
    IterHelper(t1, t2, (0, init)).collect()
}

fn expand_argsn(inp: (i32, Vec<i32>)) -> Vec<Option<i32>> {
    let inp1 = (1, inp.0 + 1);
    let (_, inp2) = inp;
    expand_argsn_gen(inp1, inp2)
}

impl<T: TableSchemas> CommonSQLQuery<T, sqlx::Postgres> for InsertTable {
    fn sql_statement() -> String {
        //not good, sql statements can be cached or formed by marco in advance
        //fix it later ...
        let sql = format!(
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
        );
        sql
    }
}

pub struct InsertTableBatch {}

impl<'a, T, DB> BindQueryArg<'a, DB> for [T]
where
    DB: sqlx::Database,
    T: BindQueryArg<'a, DB>,
{
    fn bind_args<'g, 'q: 'g>(&'q self, arg: &mut impl sqlx::Arguments<'g, Database = DB>)
    where
        'a: 'q,
    {
        self.iter().for_each(move |t| t.bind_args(arg))
    }
}

impl<DB: sqlx::Database> FinalQuery<DB> for InsertTableBatch {
    fn query_final(res: Result<DB::QueryResult, sqlx::Error>) -> Result<SqlResultExt, sqlx::Error> {
        res?;
        Ok(SqlResultExt::QueryResult)
    }
}

impl<T: TableSchemas> CommonSQLQuery<[T], sqlx::Postgres> for InsertTableBatch {
    fn sql_statement() -> String {
        <InsertTable as CommonSQLQuery<T, sqlx::Postgres>>::sql_statement()
    }
    fn sql_statement_rt(t: &[T]) -> String {
        let s = <Self as CommonSQLQuery<[T], sqlx::Postgres>>::sql_statement();
        (1..(t.len() as i32))
            .map(|i| (i * T::ARGN + 1, (i + 1) * T::ARGN + 1))
            .fold(s, |acc, rg| {
                acc + ",("
                    + &expand_argsn_gen(rg, T::default_argsn())
                        .iter()
                        .map(|i| match i {
                            Some(i) => format!("${}", i),
                            None => String::from("DEFAULT"),
                        })
                        .fold(String::new(), |acc, s| if acc.is_empty() { s } else { acc + "," + &s })
                    + ")"
            })
            + " ON CONFLICT DO NOTHING"
    }
}

pub trait CommonSQLQueryBatch<T: Sized, DB: sqlx::Database>: CommonSQLQuery<[T], DB> + FinalQuery<DB> {
    type ElementQueryType: CommonSQLQuery<T, DB> + FinalQuery<DB>;
}

impl<T: Sized + TableSchemas> CommonSQLQueryBatch<T, sqlx::Postgres> for InsertTableBatch {
    type ElementQueryType = InsertTable;
}

impl<'a, U, QT, DB> SqlxAction<'a, QT, DB> for [U]
where
    QT: CommonSQLQueryBatch<U, DB>,
    U: SqlxAction<'a, <QT as CommonSQLQueryBatch<U, DB>>::ElementQueryType, DB>,
    DB: CommonSQLQueryWithBind,
{
}

impl InsertTableBatch {
    pub async fn sql_query_fine<'c, 'a, Q, C, DB>(qr_v: &'a [Q], conn: &'c mut C) -> Result<SqlResultExt, (Vec<Q>, sqlx::Error)>
    where
        DB: CommonSQLQueryWithBind,
        for<'r> &'r mut C: sqlx::Executor<'r, Database = DB>,
        C: std::borrow::BorrowMut<C> + Send,
        Q: Clone,
        [Q]: SqlxAction<'a, Self, DB>,
        Self: CommonSQLQuery<[Q], DB>,
    {
        if qr_v.is_empty() {
            return Ok(SqlResultExt::Issue((0, "No element for insert")));
        }
        //recursive in async is more difficult so put it in the loop

        let mut qr_vm = qr_v;

        //we split the whole array into a group arrys with lengh = 2^n (or less 8)
        //to reduce the number of cached prepare statement (which is default)
        while qr_vm.len() >= 8 {
            for n in (3..11).rev() {
                if qr_vm.len() >= (1 << n) {
                    let qr_used = &qr_vm[..(1 << n)];
                    //log::debug!("batch {} queries", qr_used.len());
                    if let Err(e) = Self::sql_query(qr_used, &mut *conn).await {
                        return Err((qr_vm.to_vec(), e));
                    }
                    qr_vm = &qr_vm[(1 << n)..];
                    break;
                }
            }
        }

        if !qr_vm.is_empty() {
            //log::debug!("batch {} queries", qr_vm.len());
            if let Err(e) = Self::sql_query(qr_vm, &mut *conn).await {
                return Err((qr_vm.to_vec(), e));
            }
        }

        Ok(SqlResultExt::QueryResult)
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn gen_insert_str(i: (i32, Vec<i32>)) -> String {
        expand_argsn(i)
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

    struct TestSchema {}

    impl TableSchemas for TestSchema {
        const ARGN: i32 = 3;
        fn table_name() -> &'static str {
            "just_test"
        }
    }

    #[test]
    fn table_statement() {
        assert_eq!(
            <InsertTable as CommonSQLQuery<TestSchema, sqlx::Postgres>>::sql_statement(),
            "INSERT INTO just_test VALUES ($1,$2,$3)"
        );

        assert_eq!(
            <InsertTable as CommonSQLQuery<TestSchema, sqlx::Postgres>>::sql_statement_rt(&TestSchema {}),
            "INSERT INTO just_test VALUES ($1,$2,$3)"
        );

        let testvec = [TestSchema {}, TestSchema {}];

        assert_eq!(
            <InsertTableBatch as CommonSQLQuery<[TestSchema], sqlx::Postgres>>::sql_statement_rt(&testvec),
            "INSERT INTO just_test VALUES ($1,$2,$3),($4,$5,$6) ON CONFLICT DO NOTHING"
        );

        let testsingle = [TestSchema {}];
        assert_eq!(
            <InsertTableBatch as CommonSQLQuery<[TestSchema], sqlx::Postgres>>::sql_statement_rt(&testsingle),
            "INSERT INTO just_test VALUES ($1,$2,$3) ON CONFLICT DO NOTHING"
        );
    }
}
