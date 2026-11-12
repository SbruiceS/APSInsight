# frozen_string_literal: true

class CreateDatasetColumns < ActiveRecord::Migration[7.2]
  def change
    create_table :dataset_columns do |t|
      t.references :dataset_schema, null: false, foreign_key: true
      t.string :name, null: false
      t.string :data_type, null: false
      t.string :unit
      t.text :description
      t.boolean :nullable, default: true
      t.integer :position
      t.timestamps
    end
    add_index :dataset_columns, [:dataset_schema_id, :name], unique: true
  end
end
