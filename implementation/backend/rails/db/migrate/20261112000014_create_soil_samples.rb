# frozen_string_literal: true

class CreateSoilSamples < ActiveRecord::Migration[7.2]
  def change
    create_table :soil_samples do |t|
      t.references :field, foreign_key: true
      t.references :plot, foreign_key: true
      t.references :dataset, foreign_key: true
      t.datetime :sampled_at, null: false
      t.decimal :depth_cm, precision: 8, scale: 2
      t.string :sample_code
      t.decimal :latitude, precision: 10, scale: 7
      t.decimal :longitude, precision: 10, scale: 7
      t.jsonb :metadata, default: {}
      t.timestamps
    end
    add_index :soil_samples, :sampled_at
    add_index :soil_samples, :sample_code
  end
end
