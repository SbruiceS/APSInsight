# frozen_string_literal: true

class CreateImports < ActiveRecord::Migration[7.2]
  def change
    create_table :imports do |t|
      t.references :dataset, null: false, foreign_key: true
      t.references :project, null: false, foreign_key: true
      t.string :source_type, null: false
      t.string :status, null: false, default: "pending"
      t.string :original_filename
      t.jsonb :diagnostics, default: {}
      t.text :error_message
      t.timestamps
    end
    add_index :imports, :status
  end
end
