# frozen_string_literal: true

class CreatePlots < ActiveRecord::Migration[7.2]
  def change
    create_table :plots do |t|
      t.references :field, null: false, foreign_key: true
      t.string :name, null: false
      t.decimal :area_hectares, precision: 10, scale: 4
      t.timestamps
    end
  end
end
