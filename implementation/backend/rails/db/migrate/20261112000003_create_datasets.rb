# frozen_string_literal: true

class CreateDatasets < ActiveRecord::Migration[7.2]
  def change
    create_table :datasets do |t|
      t.references :project, null: false, foreign_key: true
      t.references :organization, null: false, foreign_key: true
      t.string :name, null: false
      t.string :slug, null: false
      t.string :status, null: false, default: "draft"
      t.text :description
      t.jsonb :metadata, default: {}
      t.timestamps
    end
    add_index :datasets, [:project_id, :slug], unique: true
    add_index :datasets, :status
  end
end
