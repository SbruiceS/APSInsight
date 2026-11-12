# frozen_string_literal: true

class CreateDataQualityReports < ActiveRecord::Migration[7.2]
  def change
    create_table :data_quality_reports do |t|
      t.references :dataset, null: false, foreign_key: true
      t.references :dataset_version, foreign_key: true
      t.references :import, foreign_key: true
      t.string :status, null: false, default: "pending"
      t.jsonb :summary, default: {}
      t.jsonb :issues, default: []
      t.integer :row_count
      t.integer :null_count
      t.timestamps
    end
    add_index :data_quality_reports, :status
  end
end
