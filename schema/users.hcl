schema "public" {
  comment = "A schema comment"
}

table "users" {
  schema = schema.public
  column "id" {
    type = int
  }
  column "name" {
    type = varchar(255)
  }
  column "manager_id" {
    type = int
  }
  primary_key {
    columns = [
      column.id
    ]
  }
  index "idx_users_name" {
    columns = [
      column.name
    ]
    unique = true
  }
  foreign_key "manager_fk" {
    columns = [column.manager_id]
    ref_columns = [column.id]
    on_delete = CASCADE
    on_update = NO_ACTION
  }
}
