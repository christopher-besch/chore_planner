use crate::{
    bot::{MessagableBot, ReplyMsg},
    command::handle_next_msg,
    db::{rating::RATING_OPTIONS, Db},
    test_bot::TestBot,
    week::Week,
};

use std::collections::HashMap;

#[tokio::test]
async fn test_bot_messaging() {
    let mut db = prepare_db().await;
    let mut bot = TestBot {
        to_send_msgs: vec!["@chore_planner_bot tenant list".to_string()].into_iter(),
        expected_msgs: vec![Ok(ReplyMsg::from_mono(
            r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |  Alex   | -0.58 |
|      |  @alex  |       |
+------+---------+-------+
| M403 | Jonas   | 0.50  |
|      | @jonas  | 1.00  |
+------+---------+-------+
| M404 | Thomas  | 0.00  |
|      |         |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | 0.50  |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | 0.17  |
|      |         | 7.33  |
+------+---------+-------+
| M409 |  Bob    | -0.58 |
|      |  @bob   |       |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 |         |       |
|      |         |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+"#,
        ))]
        .into_iter(),
        expected_polls: vec![],
        to_send_polls: vec![],
        next_poll_id: 0,
    };
    let msg = bot.next_msg().await.unwrap();
    handle_next_msg(&mut db, &mut bot, &msg).await;
}

#[tokio::test]
async fn test_bot_polling() {
    let mut db = prepare_db().await;
    let mut bot = TestBot {
        to_send_msgs: vec![].into_iter(),
        expected_msgs: vec![].into_iter(),
        expected_polls: vec![
            (
                "How well did Bob do the Spüldienst on 33/2024?".to_string(),
                RATING_OPTIONS.iter().map(|r| r.to_string()).collect(),
            ),
            (
                "How well did Bob do the Mülldienst on 33/2024?".to_string(),
                RATING_OPTIONS.iter().map(|r| r.to_string()).collect(),
            ),
        ],
        to_send_polls: vec![
            vec![("1 something".to_string(), 7)],
            vec![
                ("3 something else ".to_string(), 10),
                ("8 something else else".to_string(), 10),
            ],
        ],
        next_poll_id: 0,
    };
    db.weeks_to_plan = 1;
    db.update_plan(|t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();

    db.week = Week::from_db(db.week.db_week() + 1);
    db.create_rating_polls(&mut bot).await.unwrap();
    // The second invocation shouldn't do anything.
    db.create_rating_polls(&mut bot).await.unwrap();
    db.stop_rating_polls(&mut bot).await.unwrap();

    let out = db
        .list_plan(Some(Week::new(32, 2024).unwrap()))
        .await
        .unwrap();
    // Don't forget that some ratings have been created before this test.
    assert_eq!(
        out.mono_msg,
        r#"# Chores
## Spüldienst
Times performed: 4
Clean the kitchen.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 32/2024 | Jonas  |  1.00  |
+---------+--------+--------+
| 33/2024 |  Bob   |  1.00  |
+---------+--------+--------+


## Mülldienst
Times performed: 4
Take out the trash.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 32/2024 |  Alex  |        |
+---------+--------+--------+
| 33/2024 |  Bob   |  5.50  |
+---------+--------+--------+"#
    );

    let out = db.list_tenants().await.unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |  Alex   | -1.12 |
|      |  @alex  |       |
+------+---------+-------+
| M403 | Jonas   | -0.03 |
|      | @jonas  | 1.00  |
+------+---------+-------+
| M404 | Thomas  | -0.53 |
|      |         |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | 0.30  |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | -0.03 |
|      |         | 7.33  |
+------+---------+-------+
| M409 |  Bob    | 1.42  |
|      |  @bob   | 4.33  |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 |         |       |
|      |         |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+"#
    );
}

#[tokio::test]
async fn test_get_rooms_tenant_with_tenant() {
    let mut db = prepare_db().await;
    let out = db.get_rooms_tenant("M403").await.unwrap();
    assert_eq!(out, Some("Jonas".to_string()));
}
#[tokio::test]
async fn test_get_rooms_tenant_without_tenant() {
    let mut db = prepare_db().await;
    let out = db.get_rooms_tenant("M413").await.unwrap();
    assert_eq!(out, None);
}

#[tokio::test]
async fn test_get_tenants_room_with_room() {
    let mut db = prepare_db().await;
    let out = db.get_tenants_room("Jonas").await.unwrap();
    assert_eq!(out, Some("M403".to_string()));
}
#[tokio::test]
async fn test_get_tenants_room_without_room() {
    let mut db = prepare_db().await;
    let out = db.get_tenants_room("Chris").await.unwrap();
    assert_eq!(out, None);
}

#[tokio::test]
async fn test_list_tenants() {
    let mut db = prepare_db().await;
    let out = db.list_tenants().await.unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |  Alex   | -0.58 |
|      |  @alex  |       |
+------+---------+-------+
| M403 | Jonas   | 0.50  |
|      | @jonas  | 1.00  |
+------+---------+-------+
| M404 | Thomas  | 0.00  |
|      |         |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | 0.50  |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | 0.17  |
|      |         | 7.33  |
+------+---------+-------+
| M409 |  Bob    | -0.58 |
|      |  @bob   |       |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 |         |       |
|      |         |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+"#
    );
}

#[tokio::test]
async fn test_move_in_old() {
    let mut db = prepare_db().await;
    let out = db
        .move_in("ChRiS", &Some("@chris".to_string()), "M412", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |  Alex   | -0.58 |
|      |  @alex  |       |
+------+---------+-------+
| M403 | Jonas   | 0.50  |
|      | @jonas  | 1.00  |
+------+---------+-------+
| M404 | Thomas  | 0.00  |
|      |         |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | 0.50  |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | 0.17  |
|      |         | 7.33  |
+------+---------+-------+
| M409 |  Bob    | -0.58 |
|      |  @bob   |       |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 | Chris   | 0.00  |
|      | @chris  |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+"#
    );
}
#[tokio::test]
async fn test_move_in_old_without_tag() {
    let mut db = prepare_db().await;
    let out = db
        .move_in("CHRis", &None, "M412", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |  Alex   | -0.58 |
|      |  @alex  |       |
+------+---------+-------+
| M403 | Jonas   | 0.50  |
|      | @jonas  | 1.00  |
+------+---------+-------+
| M404 | Thomas  | 0.00  |
|      |         |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | 0.50  |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | 0.17  |
|      |         | 7.33  |
+------+---------+-------+
| M409 |  Bob    | -0.58 |
|      |  @bob   |       |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 | Chris   | 0.00  |
|      | @chris  |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+"#
    );
}
#[tokio::test]
async fn test_update_tag() {
    let mut db = prepare_db().await;
    db.move_in("THomas", &Some("@thomas".to_string()), "M412", |t, w| {
        format!("testing testing, {}, {}", t, w)
    })
    .await
    .unwrap();
    let out = db.list_tenants().await.unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |  Alex   | -0.58 |
|      |  @alex  |       |
+------+---------+-------+
| M403 | Jonas   | 0.50  |
|      | @jonas  | 1.00  |
+------+---------+-------+
| M404 | Thomas  | 0.00  |
|      | @thomas |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | 0.50  |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | 0.17  |
|      |         | 7.33  |
+------+---------+-------+
| M409 |  Bob    | -0.58 |
|      |  @bob   |       |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 |         |       |
|      |         |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+"#
    );
}
#[tokio::test]
async fn test_move_in_new() {
    let mut db = prepare_db().await;
    let out = db
        .move_in("yuu", &Some("@yuu".to_string()), "M412", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |  Alex   | -0.58 |
|      |  @alex  |       |
+------+---------+-------+
| M403 | Jonas   | 0.50  |
|      | @jonas  | 1.00  |
+------+---------+-------+
| M404 | Thomas  | 0.00  |
|      |         |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | 0.50  |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | 0.17  |
|      |         | 7.33  |
+------+---------+-------+
| M409 |  Bob    | -0.58 |
|      |  @bob   |       |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 |  Yuu    | 0.00  |
|      |  @yuu   |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+"#
    );
    assert_eq!(
        db.get_rooms_tenant("M412").await.unwrap(),
        Some("Yuu".to_string())
    );
}
#[tokio::test]
async fn test_move_in_fails() {
    let mut db = prepare_db().await;
    let out = db
        .move_in("Thomas", &None, "M412", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        "the tenant Thomas is currenlty living in M404, move them out of there first"
    );
    let out = db
        .move_in("Jörg", &None, "M402", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await
        .unwrap();
    assert_eq!(out.mono_msg, "Alex is living in room M402");
}

#[tokio::test]
async fn test_move_out() {
    let mut db = prepare_db().await;
    let out = db
        .move_out("jonas", |t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |  Alex   | -0.58 |
|      |  @alex  |       |
+------+---------+-------+
| M403 |         |       |
|      |         |       |
+------+---------+-------+
| M404 | Thomas  | 0.00  |
|      |         |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | 0.50  |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | 0.17  |
|      |         | 7.33  |
+------+---------+-------+
| M409 |  Bob    | -0.58 |
|      |  @bob   |       |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 |         |       |
|      |         |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+"#
    );
}

#[tokio::test]
async fn test_create_room() {
    let mut db = prepare_db().await;
    let out = db.create_room("M400").await.unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M400 |         |       |
|      |         |       |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |  Alex   | -0.58 |
|      |  @alex  |       |
+------+---------+-------+
| M403 | Jonas   | 0.50  |
|      | @jonas  | 1.00  |
+------+---------+-------+
| M404 | Thomas  | 0.00  |
|      |         |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | 0.50  |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | 0.17  |
|      |         | 7.33  |
+------+---------+-------+
| M409 |  Bob    | -0.58 |
|      |  @bob   |       |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 |         |       |
|      |         |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+"#
    );
}

#[tokio::test]
async fn test_list_plan() {
    let mut db = prepare_db().await;
    let out = db.list_plan(Some(Week::from_db(0))).await.unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Chores
## Spüldienst
Times performed: 3
Clean the kitchen.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 30/2024 |  Till  |  7.33  |
+---------+--------+--------+
| 31/2024 |  Olli  |  4.00  |
+---------+--------+--------+
| 32/2024 | Jonas  |  1.00  |
+---------+--------+--------+


## Mülldienst
Times performed: 3
Take out the trash.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 30/2024 | Jonas  |        |
+---------+--------+--------+
| 31/2024 |  Bob   |        |
+---------+--------+--------+
| 32/2024 |  Alex  |        |
+---------+--------+--------+"#
    );
}

#[tokio::test]
async fn test_create_chore() {
    let mut db = prepare_db().await;
    let out = db
        .create_chore("Wash the Sprouts", "Make the sprouts happy.", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Chores
## Spüldienst
Times performed: 3
Clean the kitchen.

### Plan
+------+--------+--------+
| week | tenant | rating |
+------+--------+--------+


## Mülldienst
Times performed: 3
Take out the trash.

### Plan
+------+--------+--------+
| week | tenant | rating |
+------+--------+--------+


## Wash the Sprouts
Times performed: 0
Make the sprouts happy.

### Plan
+------+--------+--------+
| week | tenant | rating |
+------+--------+--------+"#
    );
}

#[tokio::test]
async fn test_set_chore_active_state() {
    let mut db = prepare_db().await;
    db.create_chore("Wash the Sprouts", "Make the sprouts happy.", |t, w| {
        format!("testing testing, {}, {}", t, w)
    })
    .await
    .unwrap();
    {
        let out = db
            .set_chore_active_state("Mülldienst", false, |t, w| {
                format!("testing testing, {}, {}", t, w)
            })
            .await
            .unwrap();
        assert_eq!(
            out.mono_msg,
            r#"# Chores
## Spüldienst
Times performed: 3
Clean the kitchen.

### Plan
+------+--------+--------+
| week | tenant | rating |
+------+--------+--------+


## Wash the Sprouts
Times performed: 0
Make the sprouts happy.

### Plan
+------+--------+--------+
| week | tenant | rating |
+------+--------+--------+"#
        );
    }
    {
        let out = db
            .set_chore_active_state("Wash the Sprouts", false, |t, w| {
                format!("testing testing, {}, {}", t, w)
            })
            .await
            .unwrap();
        assert_eq!(
            out.mono_msg,
            r#"# Chores
## Spüldienst
Times performed: 3
Clean the kitchen.

### Plan
+------+--------+--------+
| week | tenant | rating |
+------+--------+--------+"#
        );
    }
    {
        let out = db
            .set_chore_active_state("Mülldienst", true, |t, w| {
                format!("testing testing, {}, {}", t, w)
            })
            .await
            .unwrap();
        assert_eq!(
            out.mono_msg,
            r#"# Chores
## Spüldienst
Times performed: 3
Clean the kitchen.

### Plan
+------+--------+--------+
| week | tenant | rating |
+------+--------+--------+


## Mülldienst
Times performed: 3
Take out the trash.

### Plan
+------+--------+--------+
| week | tenant | rating |
+------+--------+--------+"#
        );
    }
    {
        assert!(db
            .set_chore_active_state("Typo", true, |t, w| format!(
                "testing testing, {}, {}",
                t, w
            ))
            .await
            .is_err());
    }
}

#[tokio::test]
async fn test_list_exemptions() {
    let mut db = prepare_db().await;
    let out = db.list_exemptions().await.unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Exemptions

+------------------+------------+---------+
|      reason      |   chores   | tenants |
+------------------+------------+---------+
|       God        | Mülldienst |         |
|                  | Spüldienst |         |
+------------------+------------+---------+
| Getränkeminister | Mülldienst |         |
+------------------+------------+---------+
| Bestandsminister | Mülldienst |  Olli   |
|                  |            |  Till   |
|                  |            |  Chris  |
+------------------+------------+---------+"#
    );
}
#[tokio::test]
async fn test_create_exemptions() {
    let mut db = prepare_db().await;
    let out = db
        .create_exemption_reason("Elon", &vec!["Spüldienst".to_string()])
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Exemptions

+------------------+------------+---------+
|      reason      |   chores   | tenants |
+------------------+------------+---------+
|       God        | Mülldienst |         |
|                  | Spüldienst |         |
+------------------+------------+---------+
| Getränkeminister | Mülldienst |         |
+------------------+------------+---------+
| Bestandsminister | Mülldienst |  Olli   |
|                  |            |  Till   |
|                  |            |  Chris  |
+------------------+------------+---------+
|       Elon       | Spüldienst |         |
+------------------+------------+---------+"#
    );
}
#[tokio::test]
async fn test_change_exemption_reason_remove() {
    let mut db = prepare_db().await;
    let out = db
        .change_exemption_reason(
            "God",
            // PigeonFeeder is inactive
            &vec!["PigeonFeeder".to_string(), "Spüldienst".to_string()],
            |t, w| format!("testing testing, {}, {}", t, w),
        )
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Exemptions

+------------------+------------+---------+
|      reason      |   chores   | tenants |
+------------------+------------+---------+
|       God        | Spüldienst |         |
+------------------+------------+---------+
| Getränkeminister | Mülldienst |         |
+------------------+------------+---------+
| Bestandsminister | Mülldienst |  Olli   |
|                  |            |  Till   |
|                  |            |  Chris  |
+------------------+------------+---------+"#
    );
}
#[tokio::test]
async fn test_change_exemption_reason_remove_and_add() {
    let mut db = prepare_db().await;
    let out = db
        .change_exemption_reason(
            "Getränkeminister",
            &vec!["Spüldienst".to_string()],
            |t, w| format!("testing testing, {}, {}", t, w),
        )
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Exemptions

+------------------+------------+---------+
|      reason      |   chores   | tenants |
+------------------+------------+---------+
|       God        | Mülldienst |         |
|                  | Spüldienst |         |
+------------------+------------+---------+
| Getränkeminister | Spüldienst |         |
+------------------+------------+---------+
| Bestandsminister | Mülldienst |  Olli   |
|                  |            |  Till   |
|                  |            |  Chris  |
+------------------+------------+---------+"#
    );
}
#[tokio::test]
async fn test_change_exemption_invalid_reason() {
    let mut db = prepare_db().await;
    let out = db
        .change_exemption_reason("Invalid", &vec![], |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await;
    if let Err(e) = out {
        assert_eq!(e.to_string(), "the ExemptionReason Invalid doesn't exist")
    } else {
        panic!();
    }
}
#[tokio::test]
async fn test_grant_exemptions() {
    let mut db = prepare_db().await;
    let out = db
        .grant_exemption("God", "Chris", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Exemptions

+------------------+------------+---------+
|      reason      |   chores   | tenants |
+------------------+------------+---------+
|       God        | Mülldienst |  Chris  |
|                  | Spüldienst |         |
+------------------+------------+---------+
| Getränkeminister | Mülldienst |         |
+------------------+------------+---------+
| Bestandsminister | Mülldienst |  Olli   |
|                  |            |  Till   |
|                  |            |  Chris  |
+------------------+------------+---------+"#
    );
    // can't add again
    let res = db
        .grant_exemption("God", "Chris", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await;
    if let Err(e) = res {
        assert_eq!(e.to_string(), "tenant is already exempt");
    } else {
        panic!();
    }

    // can't add again even next week
    db.set_week(Week::new(34, 2024).unwrap());
    let res = db
        .grant_exemption("God", "Chris", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await;
    if let Err(e) = res {
        assert_eq!(e.to_string(), "tenant is already exempt");
    } else {
        panic!();
    }
}
#[tokio::test]
async fn test_revoke_exemptions() {
    let mut db = prepare_db().await;
    let out = db
        .revoke_exemption("Bestandsminister", "Chris", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Exemptions

+------------------+------------+---------+
|      reason      |   chores   | tenants |
+------------------+------------+---------+
|       God        | Mülldienst |         |
|                  | Spüldienst |         |
+------------------+------------+---------+
| Getränkeminister | Mülldienst |         |
+------------------+------------+---------+
| Bestandsminister | Mülldienst |  Olli   |
|                  |            |  Till   |
+------------------+------------+---------+"#
    );

    // can't revoke again
    let res = db
        .revoke_exemption("Bestandsminister", "Chris", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await;
    if let Err(e) = res {
        assert_eq!(e.to_string(), "tenant is not exempt");
    } else {
        panic!();
    }

    // can't revoke again even next week
    db.set_week(Week::new(34, 2024).unwrap());
    let res = db
        .revoke_exemption("Bestandsminister", "Chris", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await;
    if let Err(e) = res {
        assert_eq!(e.to_string(), "tenant is not exempt");
    } else {
        panic!();
    }
}

#[tokio::test]
async fn test_get_available_tenants() {
    let mut db = prepare_db().await;
    let out = db
        .get_available_tenants(Week::new(34, 2024).unwrap(), "Spüldienst")
        .await
        .unwrap();
    assert_eq!(
        out,
        vec![
            ("Alex".to_string(), -0.75),
            ("Bob".to_string(), -0.75),
            ("Thomas".to_string(), 0.0),
            ("Jonas".to_string(), 0.5),
            ("Olli".to_string(), 0.5),
            ("Till".to_string(), 0.5)
        ]
    );
}

#[tokio::test]
async fn test_calc_tenant_distribution() {
    let mut db = prepare_db().await;
    let tenants = vec![
        ("Alex".to_string(), -0.75),
        ("Bob".to_string(), -0.75),
        ("Thomas".to_string(), 0.0),
        ("Jonas".to_string(), 0.5),
        ("Olli".to_string(), 0.5),
        ("Till".to_string(), 0.5),
    ];
    assert_eq!(
        db.calc_tenant_distribution(tenants),
        vec![
            (0.21666666666666665),
            (0.21666666666666665),
            (0.16666666666666666),
            (0.13333333333333333),
            (0.13333333333333333),
            (0.13333333333333333)
        ]
    );
}

#[tokio::test]
async fn test_calc_tenant_distribution_all_zero() {
    let mut db = prepare_db().await;
    let tenants = vec![
        ("Alex".to_string(), 0.0),
        ("Bob".to_string(), 0.0),
        ("Thomas".to_string(), 0.0),
        ("Jonas".to_string(), 0.0),
        ("Olli".to_string(), 0.0),
        ("Till".to_string(), 0.0),
    ];
    assert_eq!(
        db.calc_tenant_distribution(tenants),
        vec![
            (0.16666666666666666),
            (0.16666666666666666),
            (0.16666666666666666),
            (0.16666666666666666),
            (0.16666666666666666),
            (0.16666666666666666)
        ]
    );
}

#[tokio::test]
async fn test_choose_tenant() {
    let mut db = prepare_db().await;

    let tenants = vec![
        ("Alex".to_string(), -0.75),
        ("Bob".to_string(), -0.75),
        ("Thomas".to_string(), 0.0),
        ("Jonas".to_string(), 0.5),
        ("Olli".to_string(), 0.5),
        ("Till".to_string(), 0.5),
    ];
    let mut map: HashMap<String, u32> = HashMap::new();

    let n = 100000;
    for _ in 0..n {
        let (tenant, score, prob) = db.choose_tenant(tenants.clone()).await.unwrap();
        match tenant.as_ref() {
            "Alex" => {
                assert_eq!(score, -0.75);
                assert_eq!(prob, 0.21666666666666665);
            }
            "Bob" => {
                assert_eq!(score, -0.75);
                assert_eq!(prob, 0.21666666666666665);
            }
            "Thomas" => {
                assert_eq!(score, 0.0);
                assert_eq!(prob, 0.16666666666666666);
            }
            "Jonas" => {
                assert_eq!(score, 0.5);
                assert_eq!(prob, 0.13333333333333333);
            }
            "Olli" => {
                assert_eq!(score, 0.5);
                assert_eq!(prob, 0.13333333333333333);
            }
            "Till" => {
                assert_eq!(score, 0.5);
                assert_eq!(prob, 0.13333333333333333);
            }
            _ => {
                panic!();
            }
        }
        *map.entry(tenant).or_default() += 1;
    }
    for (tenant, count) in map {
        let prob = count as f64 / n as f64;
        const ETA: f64 = 0.01;
        match tenant.as_ref() {
            "Alex" => {
                assert!((prob - 0.21666666666666665).abs() < ETA);
            }
            "Bob" => {
                assert!((prob - 0.21666666666666665).abs() < ETA);
            }
            "Thomas" => {
                assert!((prob - 0.16666666666666666).abs() < ETA);
            }
            "Jonas" => {
                assert!((prob - 0.13333333333333333).abs() < ETA);
            }
            "Olli" => {
                assert!((prob - 0.13333333333333333).abs() < ETA);
            }
            "Till" => {
                assert!((prob - 0.13333333333333333).abs() < ETA);
            }
            _ => {
                panic!();
            }
        }
    }
}

#[tokio::test]
async fn test_get_weeks_to_plan() {
    let mut db = prepare_db().await;

    let out = db.get_weeks_to_plan().await.unwrap();
    assert!(out.is_empty());

    db.weeks_to_plan = 5;
    let out = db.get_weeks_to_plan().await.unwrap();
    assert_eq!(
        out,
        vec![
            (Week::from_db(2850), "Spüldienst".to_string()),
            (Week::from_db(2850), "Mülldienst".to_string()),
            (Week::from_db(2851), "Spüldienst".to_string()),
            (Week::from_db(2851), "Mülldienst".to_string()),
            (Week::from_db(2852), "Spüldienst".to_string()),
            (Week::from_db(2852), "Mülldienst".to_string()),
            (Week::from_db(2853), "Spüldienst".to_string()),
            (Week::from_db(2853), "Mülldienst".to_string()),
            (Week::from_db(2854), "Spüldienst".to_string()),
            (Week::from_db(2854), "Mülldienst".to_string()),
        ]
    );
}

#[tokio::test]
async fn test_update_plan() {
    let mut db = prepare_db().await;
    db.weeks_to_plan = 1;
    let out = db
        .update_plan(|t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Spüldienst on 33/2024: Bob
Bob, you have been chosen for the Spüldienst on 33/2024.
According to your effective score -0.75 you've had a probability of 26% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Bob, 33/2024
Alternatively you can move out and then back in if you're on vacation.



# Mülldienst on 33/2024: Bob
Bob, you have been chosen for the Mülldienst on 33/2024.
According to your effective score 0.06 you've had a probability of 27% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Bob, 33/2024
Alternatively you can move out and then back in if you're on vacation.



# Chores
## Spüldienst
Times performed: 4
Clean the kitchen.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Bob   |        |
+---------+--------+--------+


## Mülldienst
Times performed: 4
Take out the trash.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Bob   |        |
+---------+--------+--------+"#
    );

    db.weeks_to_plan = 5;
    let out = db
        .update_plan(|t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Spüldienst on 34/2024: Olli
Olli, you have been chosen for the Spüldienst on 34/2024.
According to your effective score 0.30 you've had a probability of 13% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Olli, 34/2024
Alternatively you can move out and then back in if you're on vacation.



# Mülldienst on 34/2024: Alex
Alex, you have been chosen for the Mülldienst on 34/2024.
According to your effective score -0.25 you've had a probability of 26% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Alex, 34/2024
Alternatively you can move out and then back in if you're on vacation.



# Spüldienst on 35/2024: Olli
Olli, you have been chosen for the Spüldienst on 35/2024.
According to your effective score 1.31 you've had a probability of 16% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Olli, 35/2024
Alternatively you can move out and then back in if you're on vacation.



# Mülldienst on 35/2024: Thomas
Thomas, you have been chosen for the Mülldienst on 35/2024.
According to your effective score -0.50 you've had a probability of 37% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Thomas, 35/2024
Alternatively you can move out and then back in if you're on vacation.



# Spüldienst on 36/2024: Bob
Bob, you have been chosen for the Spüldienst on 36/2024.
According to your effective score -0.15 you've had a probability of 17% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Bob, 36/2024
Alternatively you can move out and then back in if you're on vacation.



# Mülldienst on 36/2024: Thomas
Thomas, you have been chosen for the Mülldienst on 36/2024.
According to your effective score 0.25 you've had a probability of 22% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Thomas, 36/2024
Alternatively you can move out and then back in if you're on vacation.



# Spüldienst on 37/2024: Till
Till, you have been chosen for the Spüldienst on 37/2024.
According to your effective score -0.30 you've had a probability of 17% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Till, 37/2024
Alternatively you can move out and then back in if you're on vacation.



# Mülldienst on 37/2024: Alex
Alex, you have been chosen for the Mülldienst on 37/2024.
According to your effective score 0.08 you've had a probability of 25% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Alex, 37/2024
Alternatively you can move out and then back in if you're on vacation.



# Chores
## Spüldienst
Times performed: 4
Clean the kitchen.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Bob   |        |
+---------+--------+--------+
| 34/2024 |  Olli  |        |
+---------+--------+--------+
| 35/2024 |  Olli  |        |
+---------+--------+--------+
| 36/2024 |  Bob   |        |
+---------+--------+--------+
| 37/2024 |  Till  |        |
+---------+--------+--------+


## Mülldienst
Times performed: 4
Take out the trash.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Bob   |        |
+---------+--------+--------+
| 34/2024 |  Alex  |        |
+---------+--------+--------+
| 35/2024 | Thomas |        |
+---------+--------+--------+
| 36/2024 | Thomas |        |
+---------+--------+--------+
| 37/2024 |  Alex  |        |
+---------+--------+--------+"#
    );
}

#[tokio::test]
async fn test_create_chore_update() {
    let mut db = prepare_db().await;
    db.weeks_to_plan = 5;
    db.update_plan(|t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();

    let out = db
        .create_chore("Clean the Furnace", "Do something with coal", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Clean the Furnace on 33/2024: Jonas
Jonas, you have been chosen for the Clean the Furnace on 33/2024.
According to your effective score 0.00 you've had a probability of 20% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Jonas, 33/2024
Alternatively you can move out and then back in if you're on vacation.



# Clean the Furnace on 34/2024: Bob
Bob, you have been chosen for the Clean the Furnace on 34/2024.
According to your effective score -0.20 you've had a probability of 17% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Bob, 34/2024
Alternatively you can move out and then back in if you're on vacation.



# Clean the Furnace on 35/2024: Thomas
Thomas, you have been chosen for the Clean the Furnace on 35/2024.
According to your effective score -0.24 you've had a probability of 21% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Thomas, 35/2024
Alternatively you can move out and then back in if you're on vacation.



# Clean the Furnace on 36/2024: Till
Till, you have been chosen for the Clean the Furnace on 36/2024.
According to your effective score -0.60 you've had a probability of 20% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Till, 36/2024
Alternatively you can move out and then back in if you're on vacation.



# Clean the Furnace on 37/2024: Olli
Olli, you have been chosen for the Clean the Furnace on 37/2024.
According to your effective score -0.80 you've had a probability of 23% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Olli, 37/2024
Alternatively you can move out and then back in if you're on vacation.



# Chores
## Spüldienst
Times performed: 4
Clean the kitchen.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Bob   |        |
+---------+--------+--------+
| 34/2024 |  Olli  |        |
+---------+--------+--------+
| 35/2024 |  Olli  |        |
+---------+--------+--------+
| 36/2024 |  Bob   |        |
+---------+--------+--------+
| 37/2024 |  Till  |        |
+---------+--------+--------+


## Mülldienst
Times performed: 4
Take out the trash.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Bob   |        |
+---------+--------+--------+
| 34/2024 |  Alex  |        |
+---------+--------+--------+
| 35/2024 | Thomas |        |
+---------+--------+--------+
| 36/2024 | Thomas |        |
+---------+--------+--------+
| 37/2024 |  Alex  |        |
+---------+--------+--------+


## Clean the Furnace
Times performed: 1
Do something with coal

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 | Jonas  |        |
+---------+--------+--------+
| 34/2024 |  Bob   |        |
+---------+--------+--------+
| 35/2024 | Thomas |        |
+---------+--------+--------+
| 36/2024 |  Till  |        |
+---------+--------+--------+
| 37/2024 |  Olli  |        |
+---------+--------+--------+"#
    );
}

#[tokio::test]
async fn test_exempt_update() {
    let mut db = prepare_db().await;
    db.weeks_to_plan = 5;
    db.update_plan(|t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();

    db.create_exemption_reason("Programmer", &vec!["Spüldienst".to_string()])
        .await
        .unwrap();

    let out = db
        .grant_exemption("Programmer", "Bob", |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Exemptions

+------------------+------------+---------+
|      reason      |   chores   | tenants |
+------------------+------------+---------+
|       God        | Mülldienst |         |
|                  | Spüldienst |         |
+------------------+------------+---------+
| Getränkeminister | Mülldienst |         |
+------------------+------------+---------+
| Bestandsminister | Mülldienst |  Olli   |
|                  |            |  Till   |
|                  |            |  Chris  |
+------------------+------------+---------+
|    Programmer    | Spüldienst |   Bob   |
+------------------+------------+---------+



# Spüldienst on 33/2024: Jonas
Jonas, you have been chosen for the Spüldienst on 33/2024.
According to your effective score -0.62 you've had a probability of 27% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Jonas, 33/2024
Alternatively you can move out and then back in if you're on vacation.



# Spüldienst on 36/2024: Thomas
Thomas, you have been chosen for the Spüldienst on 36/2024.
According to your effective score -1.15 you've had a probability of 22% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Thomas, 36/2024
Alternatively you can move out and then back in if you're on vacation.



# Chores
## Spüldienst
Times performed: 4
Clean the kitchen.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 | Jonas  |        |
+---------+--------+--------+
| 34/2024 |  Olli  |        |
+---------+--------+--------+
| 35/2024 |  Olli  |        |
+---------+--------+--------+
| 36/2024 | Thomas |        |
+---------+--------+--------+
| 37/2024 |  Till  |        |
+---------+--------+--------+


## Mülldienst
Times performed: 4
Take out the trash.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Bob   |        |
+---------+--------+--------+
| 34/2024 |  Alex  |        |
+---------+--------+--------+
| 35/2024 | Thomas |        |
+---------+--------+--------+
| 36/2024 | Thomas |        |
+---------+--------+--------+
| 37/2024 |  Alex  |        |
+---------+--------+--------+"#
    );
}

#[tokio::test]
async fn test_replan_update() {
    let mut db = prepare_db().await;
    db.weeks_to_plan = 5;
    db.update_plan(|t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();

    let out = db
        .replan("Thomas", Week::new(36, 2024).unwrap(), |t, w| {
            format!("testing testing, {}, {}", t, w)
        })
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Mülldienst on 36/2024: Bob
Bob, you have been chosen for the Mülldienst on 36/2024.
According to your effective score 0.06 you've had a probability of 33% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Bob, 36/2024
Alternatively you can move out and then back in if you're on vacation.



# Chores
## Spüldienst
Times performed: 4
Clean the kitchen.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Bob   |        |
+---------+--------+--------+
| 34/2024 |  Olli  |        |
+---------+--------+--------+
| 35/2024 |  Olli  |        |
+---------+--------+--------+
| 36/2024 |  Bob   |        |
+---------+--------+--------+
| 37/2024 |  Till  |        |
+---------+--------+--------+


## Mülldienst
Times performed: 4
Take out the trash.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Bob   |        |
+---------+--------+--------+
| 34/2024 |  Alex  |        |
+---------+--------+--------+
| 35/2024 | Thomas |        |
+---------+--------+--------+
| 36/2024 |  Bob   |        |
+---------+--------+--------+
| 37/2024 |  Alex  |        |
+---------+--------+--------+"#
    );
}

#[tokio::test]
async fn test_move_out_update() {
    let mut db = prepare_db().await;
    db.weeks_to_plan = 5;
    db.update_plan(|t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();

    let out = db
        .move_out("Thomas", |t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |  Alex   | -1.33 |
|      |  @alex  |       |
+------+---------+-------+
| M403 | Jonas   | -3.25 |
|      | @jonas  | 1.00  |
+------+---------+-------+
| M404 |         |       |
|      |         |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | 1.75  |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | 0.17  |
|      |         | 7.33  |
+------+---------+-------+
| M409 |  Bob    | -0.33 |
|      |  @bob   |       |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 |         |       |
|      |         |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+



# Mülldienst on 35/2024: Jonas
Jonas, you have been chosen for the Mülldienst on 35/2024.
According to your effective score -1.58 you've had a probability of 60% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Jonas, 35/2024
Alternatively you can move out and then back in if you're on vacation.



# Mülldienst on 36/2024: Jonas
Jonas, you have been chosen for the Mülldienst on 36/2024.
According to your effective score -0.61 you've had a probability of 37% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Jonas, 36/2024
Alternatively you can move out and then back in if you're on vacation.



# Chores
## Spüldienst
Times performed: 4
Clean the kitchen.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Bob   |        |
+---------+--------+--------+
| 34/2024 |  Olli  |        |
+---------+--------+--------+
| 35/2024 |  Olli  |        |
+---------+--------+--------+
| 36/2024 |  Bob   |        |
+---------+--------+--------+
| 37/2024 |  Till  |        |
+---------+--------+--------+


## Mülldienst
Times performed: 4
Take out the trash.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Bob   |        |
+---------+--------+--------+
| 34/2024 |  Alex  |        |
+---------+--------+--------+
| 35/2024 | Jonas  |        |
+---------+--------+--------+
| 36/2024 | Jonas  |        |
+---------+--------+--------+
| 37/2024 |  Alex  |        |
+---------+--------+--------+"#
    );

    let out = db
        .move_out("Bob", |t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |  Alex   | -3.25 |
|      |  @alex  |       |
+------+---------+-------+
| M403 | Jonas   | -2.17 |
|      | @jonas  | 1.00  |
+------+---------+-------+
| M404 |         |       |
|      |         |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | 1.50  |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | -0.17 |
|      |         | 7.33  |
+------+---------+-------+
| M409 |         |       |
|      |         |       |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 |         |       |
|      |         |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+



# Spüldienst on 33/2024: Alex
Alex, you have been chosen for the Spüldienst on 33/2024.
According to your effective score -1.94 you've had a probability of 30% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Alex, 33/2024
Alternatively you can move out and then back in if you're on vacation.



# Mülldienst on 33/2024: Jonas
Jonas, you have been chosen for the Mülldienst on 33/2024.
According to your effective score -0.08 you've had a probability of 60% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Jonas, 33/2024
Alternatively you can move out and then back in if you're on vacation.



# Spüldienst on 36/2024: Alex
Alex, you have been chosen for the Spüldienst on 36/2024.
According to your effective score -0.94 you've had a probability of 28% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Alex, 36/2024
Alternatively you can move out and then back in if you're on vacation.



# Chores
## Spüldienst
Times performed: 4
Clean the kitchen.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Alex  |        |
+---------+--------+--------+
| 34/2024 |  Olli  |        |
+---------+--------+--------+
| 35/2024 |  Olli  |        |
+---------+--------+--------+
| 36/2024 |  Alex  |        |
+---------+--------+--------+
| 37/2024 |  Till  |        |
+---------+--------+--------+


## Mülldienst
Times performed: 4
Take out the trash.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 | Jonas  |        |
+---------+--------+--------+
| 34/2024 |  Alex  |        |
+---------+--------+--------+
| 35/2024 | Jonas  |        |
+---------+--------+--------+
| 36/2024 | Jonas  |        |
+---------+--------+--------+
| 37/2024 |  Alex  |        |
+---------+--------+--------+"#
    );

    let out = db
        .move_out("Jonas", |t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |  Alex   | -0.08 |
|      |  @alex  |       |
+------+---------+-------+
| M403 |         |       |
|      |         |       |
+------+---------+-------+
| M404 |         |       |
|      |         |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | 1.00  |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | -0.83 |
|      |         | 7.33  |
+------+---------+-------+
| M409 |         |       |
|      |         |       |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 |         |       |
|      |         |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+



# Mülldienst on 33/2024: Alex
Alex, you have been chosen for the Mülldienst on 33/2024.
According to your effective score 0.00 you've had a probability of 100% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Alex, 33/2024
Alternatively you can move out and then back in if you're on vacation.



# Mülldienst on 35/2024: Alex
Alex, you have been chosen for the Mülldienst on 35/2024.
According to your effective score 0.00 you've had a probability of 100% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Alex, 35/2024
Alternatively you can move out and then back in if you're on vacation.



# Mülldienst on 36/2024: Alex
Alex, you have been chosen for the Mülldienst on 36/2024.
According to your effective score 0.00 you've had a probability of 100% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Alex, 36/2024
Alternatively you can move out and then back in if you're on vacation.



# Chores
## Spüldienst
Times performed: 4
Clean the kitchen.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Alex  |        |
+---------+--------+--------+
| 34/2024 |  Olli  |        |
+---------+--------+--------+
| 35/2024 |  Olli  |        |
+---------+--------+--------+
| 36/2024 |  Alex  |        |
+---------+--------+--------+
| 37/2024 |  Till  |        |
+---------+--------+--------+


## Mülldienst
Times performed: 4
Take out the trash.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Alex  |        |
+---------+--------+--------+
| 34/2024 |  Alex  |        |
+---------+--------+--------+
| 35/2024 |  Alex  |        |
+---------+--------+--------+
| 36/2024 |  Alex  |        |
+---------+--------+--------+
| 37/2024 |  Alex  |        |
+---------+--------+--------+"#
    );

    let out = db
        .move_out("Alex", |t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Tenants

+------+---------+-------+
| room | tenant  | score |
|      |         | eval  |
+------+---------+-------+
| M401 |         |       |
|      |         |       |
+------+---------+-------+
| M402 |         |       |
|      |         |       |
+------+---------+-------+
| M403 |         |       |
|      |         |       |
+------+---------+-------+
| M404 |         |       |
|      |         |       |
+------+---------+-------+
| M405 |         |       |
|      |         |       |
+------+---------+-------+
| M406 |         |       |
|      |         |       |
+------+---------+-------+
| M407 | Olli    | -0.50 |
|      | @olli69 | 4.00  |
+------+---------+-------+
| M408 |  Till   | -2.83 |
|      |         | 7.33  |
+------+---------+-------+
| M409 |         |       |
|      |         |       |
+------+---------+-------+
| M410 |         |       |
|      |         |       |
+------+---------+-------+
| M411 |         |       |
|      |         |       |
+------+---------+-------+
| M412 |         |       |
|      |         |       |
+------+---------+-------+
| M413 |         |       |
|      |         |       |
+------+---------+-------+
| M414 |         |       |
|      |         |       |
+------+---------+-------+
| M415 |         |       |
|      |         |       |
+------+---------+-------+



# Spüldienst on 33/2024: Till
Till, you have been chosen for the Spüldienst on 33/2024.
According to your effective score -1.00 you've had a probability of 60% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Till, 33/2024
Alternatively you can move out and then back in if you're on vacation.



# Spüldienst on 36/2024: Olli
Olli, you have been chosen for the Spüldienst on 36/2024.
According to your effective score 0.00 you've had a probability of 50% to be chosen.
If you're unhappy about that, type this to schedule someone else:
    testing testing, Olli, 36/2024
Alternatively you can move out and then back in if you're on vacation.



# Chores
## Spüldienst
Times performed: 4
Clean the kitchen.

### Plan
+---------+--------+--------+
|  week   | tenant | rating |
+---------+--------+--------+
| 33/2024 |  Till  |        |
+---------+--------+--------+
| 34/2024 |  Olli  |        |
+---------+--------+--------+
| 35/2024 |  Olli  |        |
+---------+--------+--------+
| 36/2024 |  Olli  |        |
+---------+--------+--------+
| 37/2024 |  Till  |        |
+---------+--------+--------+


## Mülldienst
Times performed: 3
Take out the trash.

### Plan
+------+--------+--------+
| week | tenant | rating |
+------+--------+--------+"#
    );
}

#[tokio::test]
async fn test_print_next_week_banner() {
    let mut db = prepare_db().await;
    db.weeks_to_plan = 1;
    db.update_plan(|t, w| format!("testing testing, {}, {}", t, w))
        .await
        .unwrap();
    let out = db.print_next_week_banner().await.unwrap();
    assert_eq!(
        out.mono_msg,
        r#"# Week 33/2024
Hello smart people!
We have another week and new jobs to go with it:

+------------+--------+
|    job     | worker |
+------------+--------+
| Spüldienst |  Bob   |
+------------+--------+
| Mülldienst |  Bob   |
+------------+--------+

Have a very safe and productive week."#
    );
}

async fn prepare_db() -> Db {
    let mut db = Db::new(
        "sqlite::memory:",
        Week::new(33, 2024).unwrap(),
        0,
        0.8,
        0x0DDB1A5E5BAD5EEDu64,
        false,
    )
    .await
    .unwrap();
    // let mut db = Db::new(
    //     "sqlite://chore_planner.sqlite",
    //     Week::new(33, 2024).unwrap(),
    //     0,
    //     0.8,
    //     0x0DDB1A5E5BAD5EEDu64,
    //     false,
    // )
    // .await
    // .unwrap();
    let queries = vec![
        r#"
INSERT INTO Room VALUES
    ('M401'),
    ('M402'),
    ('M403'),
    ('M404'),
    ('M405'),
    ('M406'),
    ('M407'),
    ('M408'),
    ('M409'),
    ('M410'),
    ('M411'),
    ('M412'),
    ('M413'),
    ('M414'),
    ('M415');
"#,
        r#"
INSERT INTO Tenant VALUES
    (NULL, 'Alex', '@alex'),
    (NULL, 'Bob', '@bob'),
    (NULL, 'Chris', '@chris'),
    (NULL, 'Thomas', NULL),
    (NULL, 'Jan', '@jan'),
    (NULL, 'Joachim', '@joachim'),
    (NULL, 'Stefanie', '@stef'),
    (NULL, 'Jonas', '@jonas'),
    (NULL, 'Olli', '@olli69'),
    (NULL, 'Till', NULL);
"#,
        r#"
INSERT INTO LivesIn VALUES
    -- moved in before this week, not moved out
    ((SELECT id FROM Tenant WHERE name = 'Jonas'), 'M403', 2837, NULL),
    -- moved in before this week, not moved out
    ((SELECT id FROM Tenant WHERE name = 'Olli'), 'M407', 2835, NULL),
    -- moved in before this week, not moved out
    ((SELECT id FROM Tenant WHERE name = 'Till'), 'M408', 2829, NULL),
    -- moved in last year, not moved out
    ((SELECT id FROM Tenant WHERE name = 'Alex'), 'M402', 2807, NULL),
    -- moved in this year, will move out later
    ((SELECT id FROM Tenant WHERE name = 'Bob'), 'M409', 2714, 2858),
    -- moved in two years ago, moved out this year
    ((SELECT id FROM Tenant WHERE name = 'Chris'), 'M401', 2743, 2829),
    -- moved in this week, not moved out
    ((SELECT id FROM Tenant WHERE name = 'Thomas'), 'M404', 2850, NULL),
    -- moved in this week, moved out this week
    ((SELECT id FROM Tenant WHERE name = 'Jan'), 'M405', 2850, 2850),
    -- will move in next year, won't move out
    ((SELECT id FROM Tenant WHERE name = 'Stefanie'), 'M410', 2881, 2902);
"#,
        r#"
INSERT INTO Unwilling VALUES
    -- in past
    ((SELECT id FROM Tenant WHERE name = 'Alex'), 2829),
    -- this week
    ((SELECT id FROM Tenant WHERE name = 'Thomas'), 2850),
    -- in future
    ((SELECT id FROM Tenant WHERE name = 'Bob'), 2852);
"#,
        r#"
INSERT INTO Chore VALUES
    -- not used
    (NULL, 'PigeonFeeder', 'Feed the pigeons', 0),
    -- used
    (NULL, 'Spüldienst', 'Clean the kitchen.', 1),
    -- used
    (NULL, 'Mülldienst', 'Take out the trash.', 1);
"#,
        r#"
INSERT INTO ExemptionReason VALUES
    (NULL, 'God'),
    (NULL, 'Getränkeminister'),
    (NULL, 'Bestandsminister');
"#,
        r#"
INSERT INTO TenantExemption VALUES
    ((SELECT id FROM Tenant WHERE name = 'Olli'), (SELECT id FROM ExemptionReason WHERE reason = 'Bestandsminister'), 2830, NULL),
    ((SELECT id FROM Tenant WHERE name = 'Till'), (SELECT id FROM ExemptionReason WHERE reason = 'Bestandsminister'), 2848, NULL),
    ((SELECT id FROM Tenant WHERE name = 'Chris'), (SELECT id FROM ExemptionReason WHERE reason = 'Bestandsminister'), 2829, NULL),
    ((SELECT id FROM Tenant WHERE name = 'Jan'), (SELECT id FROM ExemptionReason WHERE reason = 'Getränkeminister'), 2714, 2727);
"#,
        r#"
INSERT INTO ChoreExemption VALUES
    ((SELECT id FROM Chore WHERE name = 'Mülldienst'), (SELECT id FROM ExemptionReason WHERE reason = 'God')),
    ((SELECT id FROM Chore WHERE name = 'Spüldienst'), (SELECT id FROM ExemptionReason WHERE reason = 'God')),
    ((SELECT id FROM Chore WHERE name = 'Mülldienst'), (SELECT id FROM ExemptionReason WHERE reason = 'Getränkeminister')),
    ((SELECT id FROM Chore WHERE name = 'Mülldienst'), (SELECT id FROM ExemptionReason WHERE reason = 'Bestandsminister'));
"#,
        r#"
INSERT INTO ChoreLog VALUES
    ((SELECT id FROM Chore WHERE name = 'Mülldienst'), 2847, (SELECT id FROM Tenant WHERE name = 'Jonas'), 0, NULL),
    ((SELECT id FROM Chore WHERE name = 'Spüldienst'), 2847, (SELECT id FROM Tenant WHERE name = 'Till'), 0, NULL),
    ((SELECT id FROM Chore WHERE name = 'Mülldienst'), 2848, (SELECT id FROM Tenant WHERE name = 'Bob'), 0, NULL),
    ((SELECT id FROM Chore WHERE name = 'Spüldienst'), 2848, (SELECT id FROM Tenant WHERE name = 'Olli'), 0, NULL),
    ((SELECT id FROM Chore WHERE name = 'Mülldienst'), 2849, (SELECT id FROM Tenant WHERE name = 'Alex'), 0, NULL),
    ((SELECT id FROM Chore WHERE name = 'Spüldienst'), 2849, (SELECT id FROM Tenant WHERE name = 'Jonas'), 0, NULL);
"#,
        r#"
INSERT INTO Rating VALUES
    (NULL, (SELECT id FROM Chore WHERE name = 'Spüldienst'), 2847, 3),
    (NULL, (SELECT id FROM Chore WHERE name = 'Spüldienst'), 2847, 10),
    (NULL, (SELECT id FROM Chore WHERE name = 'Spüldienst'), 2847, 9),
    (NULL, (SELECT id FROM Chore WHERE name = 'Spüldienst'), 2848, 7),
    (NULL, (SELECT id FROM Chore WHERE name = 'Spüldienst'), 2848, 1),
    (NULL, (SELECT id FROM Chore WHERE name = 'Spüldienst'), 2849, 2),
    (NULL, (SELECT id FROM Chore WHERE name = 'Spüldienst'), 2849, 0);
"#,
    ];
    for query in queries {
        sqlx::query(query)
            .execute(&mut db.con)
            .await
            .unwrap_or_else(|e| panic! {"{:#?}\n\n{}", e, query});
    }
    db
}
