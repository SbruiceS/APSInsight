# frozen_string_literal: true

class CreateDatasetSchemas < ActiveRecord::Migration[7.2]
  def change
    create_table :dataset_schemas do |t|
      t.references :dataset, null: false, foreign_key: true
      t.integer :version, null: false
      t.jsonb :raw_schema, default: {}
      t.timestamps
    end
    add_index :dataset_schemas, [:dataset_id, :version], unique: true
  end
end
