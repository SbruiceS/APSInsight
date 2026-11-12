# frozen_string_literal: true

class CreateFields < ActiveRecord::Migration[7.2]
  def change
    create_table :fields do |t|
      t.references :farm, null: false, foreign_key: true
      t.string :name, null: false
      t.decimal :area_hectares, precision: 12, scale: 4
      t.jsonb :boundary, default: {}
      t.timestamps
    end
  end
end
