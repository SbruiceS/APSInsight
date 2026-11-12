# frozen_string_literal: true

class CreateQueryRuns < ActiveRecord::Migration[7.2]
  def change
    create_table :query_runs do |t|
      t.references :dataset, foreign_key: true
      t.references :project, foreign_key: true
      t.text :sql, null: false
      t.string :status, null: false, default: "pending"
      t.jsonb :result, default: {}
      t.text :error_message
      t.jsonb :provenance, default: {}
      t.timestamps
    end
    add_index :query_runs, :status
  end
end
