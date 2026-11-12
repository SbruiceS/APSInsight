# frozen_string_literal: true

class CreateObservations < ActiveRecord::Migration[7.2]
  def change
    create_table :observations do |t|
      t.references :soil_sample, foreign_key: true
      t.references :dataset, foreign_key: true
      t.string :variable, null: false
      t.decimal :value, precision: 18, scale: 6, null: false
      t.string :unit, null: false
      t.datetime :observed_at, null: false
      t.string :quality_flag
      t.text :methodology
      t.jsonb :uncertainty, default: {}
      t.timestamps
    end
    add_index :observations, [:variable, :observed_at]
    add_index :observations, :soil_sample_id
  end
end
