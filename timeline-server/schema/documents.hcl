table "documents" {
  schema = schema.public

  // column "integration_id" {
  //   type = uuid
  // }
  column "external_id" {
    type = int
  }
  column "created" {
    // Equals "timestamp with time zone".
    type = timestamptz
  }
  column "title" {
    type = varchar
  }
  primary_key {
    columns = [
      column.external_id,
    ]
  }
  index "idx_documents_external_id" {
    columns = [
      column.external_id,
    ]
    unique = true
  }
  // foreign_key "integration_id_fk" {
  //   columns = [column.integration_id]
  //   ref_columns = [table.integrations.column.id]
  //   on_delete = CASCADE
  //   on_update = NO_ACTION
  // }
}
