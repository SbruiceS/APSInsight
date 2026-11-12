# frozen_string_literal: true

class CreateDatasetVersions < ActiveRecord::Migration[7.2]
  def change
    create_table :dataset_versions do |t|
      t.references :dataset, null: false, foreign_key: true
      t.integer :version_number, null: false
      t.string :checksum, null: false
      t.bigint :row_count
      t.jsonb :schema_snapshot, default: {}
      t.string :storage_path
      t.timestamps
    end
    add_index :dataset_versions, [:dataset_id, :version_number], unique: true
  end
end
