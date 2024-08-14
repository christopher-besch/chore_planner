-- create tables --
CREATE TABLE Room (
    name TEXT NOT NULL,
    --
    CONSTRAINT Room_PK PRIMARY KEY (name)
);

CREATE TABLE Tenant (
    id INTEGER PRIMARY KEY,
    telegram_id TEXT NOT NULL,
    name TEXT NOT NULL,
    --
    UNIQUE (telegram_id)
);

CREATE TABLE LivesIn (
    tenant_id INTEGER NOT NULL,
    room_name TEXT NOT NULL,
    move_in_week INTEGER NOT NULL,
    move_in_year INTEGER NOT NULL,
    move_out_week INTEGER,
    move_out_year INTEGER,
    --
    CONSTRAINT LivesIn_PK PRIMARY KEY (room_name, move_in_week, move_in_year),
    CONSTRAINT LivesIn_TO_Tenant_FK FOREIGN KEY (tenant_id) REFERENCES Tenant (id),
    CONSTRAINT LivesIn_TO_Room_FK FOREIGN KEY (room_name) REFERENCES Room (name),
    UNIQUE (tenant_id, room_name, move_in_week, move_in_year)
);

CREATE TABLE Chore (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL
);

CREATE TABLE ExemptionReason (
    id INTEGER PRIMARY KEY,
    reason TEXT NOT NULL
);

CREATE TABLE TenantExemption (
    tenant_id INTEGER NOT NULL,
    exemption_reason_id INTEGER NOT NULL,
    start_week INTEGER NOT NULL,
    start_year INTEGER NOT NULL,
    end_week INTEGER,
    end_year INTEGER,
    --
    CONSTRAINT TenantExemption_PK PRIMARY KEY (tenant_id, exemption_reason_id, start_week, start_year),
    CONSTRAINT TenantExemption_TO_Tenant_FK FOREIGN KEY (tenant_id) REFERENCES Tenant (id),
    CONSTRAINT TenantExemption_TO_ExemptionReason_FK FOREIGN KEY (exemption_reason_id) REFERENCES ExemptionReason (id)
);

CREATE TABLE ChoreExemption (
    chore_id INTEGER NOT NULL,
    exemption_reason_id INTEGER NOT NULL,
    --
    CONSTRAINT ChoreExemption_PK PRIMARY KEY (chore_id, exemption_reason_id),
    CONSTRAINT ChoreExemption_TO_Tenant_FK FOREIGN KEY (chore_id) REFERENCES Chore (id),
    CONSTRAINT ChoreExemption_TO_ExemptionReason_FK FOREIGN KEY (exemption_reason_id) REFERENCES ExemptionReason (id)
);

CREATE TABLE ChoreLog (
    chore_id INTEGER NOT NULL,
    week INTEGER NOT NULL,
    year INTEGER NOT NULL,
    planned_worker INTEGER NOT NULL,
    actual_worker INTEGER,
    --
    CONSTRAINT ChoreLog_PK PRIMARY KEY (chore_id, week, year),
    CONSTRAINT ChoreLog_TO_planned_Tenant_FK FOREIGN KEY (planned_worker) REFERENCES Tenant (id),
    CONSTRAINT ChoreLog_TO_actual_Tenant_FK FOREIGN KEY (actual_worker) REFERENCES Tenant (id)
);

CREATE TABLE Rating (
    by_tenant INTEGER NOT NULL,
    for_chore_log_chore_id INTEGER NOT NULL,
    week INTEGER NOT NULL,
    year INTEGER NOT NULL,
    rating INTEGER NOT NULL,
    comment TEXT,
    --
    CONSTRAINT Rating_PK PRIMARY KEY (by_tenant, for_chore_log_chore_id, week, year),
    CONSTRAINT Rating_TO_planned_Tenant_FK FOREIGN KEY (by_tenant) REFERENCES Tenant (id),
    CONSTRAINT Rating_TO_planned_ChoreLog_FK FOREIGN KEY (for_chore_log_chore_id, week, year) REFERENCES ChoreLog (chore_id, week, year)
);


-- insert data --
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

INSERT INTO Tenant VALUES
    (NULL, '@alex', 'Alex'),
    (NULL, '@bob', 'Bob'),
    (NULL, '@chris', 'Chris'),
    (NULL, '@thomas', 'Thomas'),
    (NULL, '@jan', 'Jan'),
    (NULL, '@joachim', 'Joachim'),
    (NULL, '@stef', 'Stefanie');

INSERT INTO LivesIn VALUES
    ((SELECT id FROM Tenant WHERE name = 'Alex'), 'M402', 42, 2023, NULL, NULL),
    ((SELECT id FROM Tenant WHERE name = 'Bob'), 'M402', 1, 2022, 41, 2023),
    ((SELECT id FROM Tenant WHERE name = 'Chris'), 'M401', 42, 2021, NULL, NULL),
    ((SELECT id FROM Tenant WHERE name = 'Thomas'), 'M404', 42, 2020, NULL, NULL),
    ((SELECT id FROM Tenant WHERE name = 'Jan'), 'M405', 42, 2019, NULL, NULL);

INSERT INTO Chore VALUES
    (NULL, 'Spüldienst', 'Clean the kitchen.'),
    (NULL, 'Mülldienst', 'Take out the trash.');

INSERT INTO ExemptionReason VALUES
    (NULL, 'Getränkeminister'),
    (NULL, 'Bestandsminister');

INSERT INTO TenantExemption VALUES
    ((SELECT id FROM Tenant WHERE name = 'Chris'), (SELECT id FROM ExemptionReason WHERE reason = 'Bestandsminister'), 12, 2024, NULL, NULL),
    ((SELECT id FROM Tenant WHERE name = 'Jan'), (SELECT id FROM ExemptionReason WHERE reason = 'Getränkeminister'), 1, 2022, 14, 2022);

INSERT INTO ChoreExemption VALUES
    ((SELECT id FROM Chore WHERE name = 'Mülldienst'), (SELECT id FROM ExemptionReason WHERE reason = 'Getränkeminister')),
    ((SELECT id FROM Chore WHERE name = 'Mülldienst'), (SELECT id FROM ExemptionReason WHERE reason = 'Bestandsminister'));

INSERT INTO ChoreLog VALUES
    ((SELECT id FROM Chore WHERE name = 'Mülldienst'), 25, 2024, (SELECT id FROM Tenant WHERE name = 'Jan'), (SELECT id FROM Tenant WHERE name = 'Jan')),
    ((SELECT id FROM Chore WHERE name = 'Spüldienst'), 25, 2024, (SELECT id FROM Tenant WHERE name = 'Bob'), (SELECT id FROM Tenant WHERE name = 'Jan')),
    ((SELECT id FROM Chore WHERE name = 'Mülldienst'), 26, 2024, (SELECT id FROM Tenant WHERE name = 'Alex'), (SELECT id FROM Tenant WHERE name = 'Chris')),
    ((SELECT id FROM Chore WHERE name = 'Spüldienst'), 26, 2024, (SELECT id FROM Tenant WHERE name = 'Jan'), (SELECT id FROM Tenant WHERE name = 'Bob'));

INSERT INTO Rating VALUES
    ((SELECT id FROM Tenant WHERE name = 'Thomas'), (SELECT id FROM Chore WHERE name = 'Spüldienst'), 26, 2024, 3, 'Richtig schlecht gemacht; den Teer riecht man immernoch.');


-- some views --
CREATE VIEW JobList (name, job, week, year) AS
    SELECT tenant.name, Chore.name, ChoreLog.week, ChoreLog.year
    FROM tenant, ChoreLog, Chore
    WHERE Chore.id = ChoreLog.chore_id
    AND (ChoreLog.actual_worker = tenant.id OR (ChoreLog.actual_worker IS NULL AND ChoreLog.planned_worker = tenant.id));

CREATE VIEW UndoneJobs (name, undone_jobs_count) AS
    SELECT tenant.name, COUNT(ALL ChoreLog.chore_id)
    FROM Tenant
                                                                  -- UNKNOWN when ChoreLog.actual_worker is NULL
    LEFT JOIN ChoreLog ON Tenant.id = ChoreLog.planned_worker AND ChoreLog.actual_worker != Tenant.id
    GROUP BY tenant.id, tenant.name;
