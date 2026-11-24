//! AI Assistant conversations, messages, runs, pending approvals, and user memories.

use palveni_core::UserId;
use palveni_core::assistant::{
    AssistantMemory, AssistantMessage, AssistantRun, Block, Conversation, MessageRole, RunState,
    RunStep, RunSubject,
};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveValue::Set, ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};

use crate::convert::{now_ms, parse_id, truncate_chars, ts};
use crate::entities::{
    assistant_conversations as convs, assistant_memories as memories,
    assistant_messages as messages, assistant_runs as runs,
};
use crate::{Db, DbError};

pub struct Assistant<'a> {
    pub(crate) db: &'a Db,
}

#[derive(Clone, Debug)]
pub struct StoredAssistantRun {
    pub user_id: UserId,
    pub conversation_id: String,
    pub run: AssistantRun,
    pub pending_action_json: Option<String>,
    pub updated_at_ms: i64,
}

fn encode_json<T: serde::Serialize>(val: &T) -> Result<String, DbError> {
    serde_json::to_string(val).map_err(|e| DbError::Internal(format!("json encode failed: {e}")))
}

fn to_stored_run(row: runs::Model) -> Result<StoredAssistantRun, DbError> {
    let steps: Vec<RunStep> = serde_json::from_str(&row.steps_json)
        .map_err(|e| DbError::Corrupt(format!("corrupt run steps `{}`: {e}", row.id)))?;
    let blocks: Vec<Block> = serde_json::from_str(&row.blocks_json)
        .map_err(|e| DbError::Corrupt(format!("corrupt run blocks `{}`: {e}", row.id)))?;
    let subject: Option<RunSubject> = row
        .subject_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| DbError::Corrupt(format!("corrupt run subject `{}`: {e}", row.id)))?;
    let previews: Option<Vec<String>> = row
        .previews_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| DbError::Corrupt(format!("corrupt run previews `{}`: {e}", row.id)))?;
    let created_at = ts(row.created_at)?;
    let user_id = parse_id(&row.user_id, "assistant run user_id")?;

    Ok(StoredAssistantRun {
        user_id,
        conversation_id: row.conversation_id,
        run: AssistantRun {
            id: row.id,
            title: row.title,
            subject,
            request: row.request_text,
            previews,
            state: RunState::from_db(&row.state),
            steps,
            blocks,
            task_id: row.task_id,
            created_at,
        },
        pending_action_json: row.pending_action_json,
        updated_at_ms: row.updated_at,
    })
}

fn to_message(row: messages::Model) -> Result<AssistantMessage, DbError> {
    Ok(AssistantMessage {
        id: row.id,
        role: MessageRole::from_db(&row.role),
        text: row.text,
        created_at: ts(row.created_at)?,
        run_id: row.run_id,
    })
}

impl Assistant<'_> {
    pub async fn active_conversation(&self, user_id: UserId) -> Result<Conversation, DbError> {
        let owner = user_id.to_string();
        let existing = convs::Entity::find()
            .filter(convs::Column::UserId.eq(owner.clone()))
            .filter(convs::Column::Archived.eq(false))
            .order_by_desc(convs::Column::UpdatedAt)
            .order_by_desc(convs::Column::Id)
            .one(self.db.main())
            .await?;

        let conv_id = match existing {
            Some(row) => row.id,
            None => {
                let id = palveni_core::JobId::generate().to_string();
                let now = now_ms();
                convs::Entity::insert(convs::ActiveModel {
                    id: Set(id.clone()),
                    user_id: Set(owner),
                    archived: Set(false),
                    created_at: Set(now),
                    updated_at: Set(now),
                })
                .exec_without_returning(self.db.main())
                .await?;
                return Ok(Conversation {
                    id,
                    messages: Vec::new(),
                    runs: Vec::new(),
                });
            }
        };

        let msg_rows = messages::Entity::find()
            .filter(messages::Column::ConversationId.eq(conv_id.clone()))
            .order_by_asc(messages::Column::CreatedAt)
            .order_by_asc(messages::Column::Id)
            .all(self.db.main())
            .await?;
        let run_rows = runs::Entity::find()
            .filter(runs::Column::ConversationId.eq(conv_id.clone()))
            .order_by_asc(runs::Column::CreatedAt)
            .order_by_asc(runs::Column::Id)
            .all(self.db.main())
            .await?;

        let mut out_msgs = Vec::with_capacity(msg_rows.len());
        for m in msg_rows {
            out_msgs.push(to_message(m)?);
        }
        let mut out_runs = Vec::with_capacity(run_rows.len());
        for r in run_rows {
            out_runs.push(to_stored_run(r)?.run);
        }

        Ok(Conversation {
            id: conv_id,
            messages: out_msgs,
            runs: out_runs,
        })
    }

    pub async fn reset_conversation(&self, user_id: UserId) -> Result<Conversation, DbError> {
        let owner = user_id.to_string();
        let now = now_ms();
        let new_id = palveni_core::JobId::generate().to_string();
        let txn = self.db.main().begin().await?;

        convs::Entity::update_many()
            .col_expr(convs::Column::Archived, Expr::val(true))
            .col_expr(convs::Column::UpdatedAt, Expr::val(now))
            .filter(convs::Column::UserId.eq(owner.clone()))
            .filter(convs::Column::Archived.eq(false))
            .exec(&txn)
            .await?;

        convs::Entity::insert(convs::ActiveModel {
            id: Set(new_id.clone()),
            user_id: Set(owner),
            archived: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .exec_without_returning(&txn)
        .await?;

        txn.commit().await?;
        Ok(Conversation {
            id: new_id,
            messages: Vec::new(),
            runs: Vec::new(),
        })
    }

    pub async fn save_turn(
        &self,
        user_id: UserId,
        conversation_id: &str,
        user_message: &AssistantMessage,
        reply_message: &AssistantMessage,
        run: &AssistantRun,
        pending_action_json: Option<String>,
    ) -> Result<(), DbError> {
        let now = palveni_core::clock::to_millis(run.created_at);
        let reply_ms = now.saturating_add(1);
        let txn = self.db.main().begin().await?;

        runs::Entity::insert(runs::ActiveModel {
            id: Set(run.id.clone()),
            conversation_id: Set(conversation_id.to_owned()),
            user_id: Set(user_id.to_string()),
            title: Set(run.title.as_deref().map(|t| truncate_chars(t, 255))),
            subject_json: Set(run.subject.as_ref().map(encode_json).transpose()?),
            request_text: Set(run.request.clone()),
            previews_json: Set(run.previews.as_ref().map(encode_json).transpose()?),
            state: Set(run.state.as_str().to_owned()),
            steps_json: Set(encode_json(&run.steps)?),
            blocks_json: Set(encode_json(&run.blocks)?),
            pending_action_json: Set(pending_action_json),
            task_id: Set(run.task_id.clone()),
            created_at: Set(now),
            updated_at: Set(now),
        })
        .exec_without_returning(&txn)
        .await?;

        messages::Entity::insert_many([
            messages::ActiveModel {
                id: Set(user_message.id.clone()),
                conversation_id: Set(conversation_id.to_owned()),
                role: Set("user".to_owned()),
                text: Set(user_message.text.clone()),
                run_id: Set(None),
                created_at: Set(now),
            },
            messages::ActiveModel {
                id: Set(reply_message.id.clone()),
                conversation_id: Set(conversation_id.to_owned()),
                role: Set("assistant".to_owned()),
                text: Set(reply_message.text.clone()),
                run_id: Set(Some(run.id.clone())),
                created_at: Set(reply_ms),
            },
        ])
        .exec_without_returning(&txn)
        .await?;

        convs::Entity::update_many()
            .col_expr(convs::Column::UpdatedAt, Expr::val(reply_ms))
            .filter(convs::Column::Id.eq(conversation_id.to_owned()))
            .exec(&txn)
            .await?;

        txn.commit().await?;
        Ok(())
    }

    pub async fn get_run(
        &self,
        user_id: UserId,
        run_id: &str,
    ) -> Result<Option<StoredAssistantRun>, DbError> {
        let row = runs::Entity::find_by_id(run_id.to_owned())
            .filter(runs::Column::UserId.eq(user_id.to_string()))
            .one(self.db.main())
            .await?;
        row.map(to_stored_run).transpose()
    }

    pub async fn find_run_by_task_or_id(
        &self,
        user_id: UserId,
        id: &str,
        is_admin: bool,
    ) -> Result<Option<StoredAssistantRun>, DbError> {
        let mut cond = Condition::any()
            .add(runs::Column::Id.eq(id.to_owned()))
            .add(runs::Column::TaskId.eq(id.to_owned()));
        if !is_admin {
            cond = Condition::all()
                .add(runs::Column::UserId.eq(user_id.to_string()))
                .add(cond);
        }
        let row = runs::Entity::find()
            .filter(cond)
            .one(self.db.main())
            .await?;
        row.map(to_stored_run).transpose()
    }

    pub async fn update_run(
        &self,
        run_id: &str,
        state: RunState,
        steps: &[RunStep],
        blocks: &[Block],
        pending_action_json: Option<String>,
        task_id: Option<String>,
    ) -> Result<Option<StoredAssistantRun>, DbError> {
        let now = now_ms();
        let steps_json = encode_json(&steps)?;
        let blocks_json = encode_json(&blocks)?;
        let res = runs::Entity::update_many()
            .col_expr(runs::Column::State, Expr::val(state.as_str().to_owned()))
            .col_expr(runs::Column::StepsJson, Expr::val(steps_json))
            .col_expr(runs::Column::BlocksJson, Expr::val(blocks_json))
            .col_expr(
                runs::Column::PendingActionJson,
                Expr::val(pending_action_json),
            )
            .col_expr(runs::Column::TaskId, Expr::val(task_id))
            .col_expr(runs::Column::UpdatedAt, Expr::val(now))
            .filter(runs::Column::Id.eq(run_id.to_owned()))
            .exec(self.db.main())
            .await?;
        if res.rows_affected == 0 {
            return Ok(None);
        }
        let row = runs::Entity::find_by_id(run_id.to_owned())
            .one(self.db.main())
            .await?;
        row.map(to_stored_run).transpose()
    }

    /// Returns runs visible in the unified Tasks view for this user:
    /// runs awaiting approval OR explicitly sent to background (`task_id IS NOT NULL`).
    pub async fn task_runs(
        &self,
        user_id: UserId,
        limit: u64,
    ) -> Result<Vec<StoredAssistantRun>, DbError> {
        let rows = runs::Entity::find()
            .filter(runs::Column::UserId.eq(user_id.to_string()))
            .filter(
                Condition::any()
                    .add(runs::Column::TaskId.is_not_null())
                    .add(runs::Column::State.eq(RunState::AwaitingApproval.as_str())),
            )
            .order_by_desc(runs::Column::CreatedAt)
            .order_by_desc(runs::Column::Id)
            .limit(limit)
            .all(self.db.main())
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            out.push(to_stored_run(row)?);
        }
        Ok(out)
    }

    pub async fn memories(&self, user_id: UserId) -> Result<Vec<AssistantMemory>, DbError> {
        let rows = memories::Entity::find()
            .filter(memories::Column::UserId.eq(user_id.to_string()))
            .order_by_desc(memories::Column::CreatedAt)
            .order_by_desc(memories::Column::Id)
            .limit(100)
            .all(self.db.main())
            .await?;
        rows.into_iter()
            .map(|r| {
                Ok(AssistantMemory {
                    id: r.id,
                    content: r.content,
                    created_at: ts(r.created_at)?,
                })
            })
            .collect()
    }

    pub async fn add_memory(
        &self,
        user_id: UserId,
        content: &str,
    ) -> Result<AssistantMemory, DbError> {
        let trimmed = truncate_chars(content.trim(), 1000);
        if trimmed.is_empty() {
            return Err(DbError::Invalid("Memory content must not be empty"));
        }
        let id = palveni_core::JobId::generate().to_string();
        let now = now_ms();
        memories::Entity::insert(memories::ActiveModel {
            id: Set(id.clone()),
            user_id: Set(user_id.to_string()),
            content: Set(trimmed.clone()),
            created_at: Set(now),
        })
        .exec_without_returning(self.db.main())
        .await?;
        Ok(AssistantMemory {
            id,
            content: trimmed,
            created_at: ts(now)?,
        })
    }

    pub async fn delete_memory(&self, user_id: UserId, id: &str) -> Result<bool, DbError> {
        let res = memories::Entity::delete_many()
            .filter(memories::Column::Id.eq(id.to_owned()))
            .filter(memories::Column::UserId.eq(user_id.to_string()))
            .exec(self.db.main())
            .await?;
        Ok(res.rows_affected > 0)
    }
}
