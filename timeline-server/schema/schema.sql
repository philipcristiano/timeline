CREATE TABLE documents (
    external_id int NOT NULL,
    created timestamptz NOT NULL,
    title varchar NOT NULL,
    CONSTRAINT documents_pkey PRIMARY KEY (external_id)
);

